use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use agentmon::app::App;
use agentmon::client::{spawn_client, ClientEvent};
use agentmon::input::{handle_key, InputAction};
use agentmon::ui::render;
use agentmon_proto::default_socket_path;

/// Everything the main loop reacts to, merged onto one channel so a single
/// blocking recv drives both daemon updates and keyboard input.
enum AppEvent {
    Client(ClientEvent),
    Key(crossterm::event::KeyEvent),
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();

    let (client_tx, client_rx) = mpsc::channel();
    spawn_client(default_socket_path(), client_tx);
    let forward_tx = tx.clone();
    thread::spawn(move || {
        for event in client_rx {
            if forward_tx.send(AppEvent::Client(event)).is_err() {
                return;
            }
        }
    });

    thread::spawn(move || loop {
        match event::poll(Duration::from_millis(150)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if tx.send(AppEvent::Key(key)).is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(_) => return,
            },
            Ok(false) => {}
            Err(_) => return,
        }
    });

    let mut app = App::new();
    loop {
        terminal.draw(|frame| render(frame, &app))?;

        match rx.recv() {
            Ok(AppEvent::Client(ClientEvent::Unreachable(reason))) => app.set_unreachable(reason),
            Ok(AppEvent::Client(ClientEvent::Reconnecting)) => app.set_reconnecting(),
            Ok(AppEvent::Client(ClientEvent::Snapshot(agents))) => app.apply_snapshot(agents),
            Ok(AppEvent::Client(ClientEvent::Update(agent))) => app.apply_update(agent),
            Ok(AppEvent::Key(key)) => {
                if handle_key(key) == InputAction::Quit {
                    return Ok(());
                }
            }
            Err(_) => return Ok(()),
        }
    }
}
