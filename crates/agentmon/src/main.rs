use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use agentmon::app::App;
use agentmon::client::{spawn_client, ClientEvent};
use agentmon::init_rspec::{formatter_path, write_rspec_local};
use agentmon::input::{handle_key, InputAction};
use agentmon::ui::render;
use agentmon_proto::default_socket_path;

/// Everything the main loop reacts to, merged onto one channel so a single
/// blocking recv drives both daemon updates and keyboard input.
enum AppEvent {
    Client(ClientEvent),
    Key(crossterm::event::KeyEvent),
    Tick,
}

/// How often the tick thread wakes the main loop to redraw, so a running
/// agent's elapsed duration counts up even without a new daemon event.
const TICK_INTERVAL: Duration = Duration::from_secs(1);

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("init-rspec") {
        return run_init_rspec();
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run_init_rspec() -> std::io::Result<()> {
    let Some(formatter) = formatter_path() else {
        eprintln!("agentmon: could not determine this binary's install location");
        std::process::exit(1);
    };

    let cwd = std::env::current_dir()?;
    write_rspec_local(&cwd, &formatter)?;
    println!(
        "agentmon: wrote {} (pointing at {})",
        cwd.join(".rspec-local").display(),
        formatter.display()
    );
    Ok(())
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

    let key_tx = tx.clone();
    thread::spawn(move || loop {
        match event::poll(Duration::from_millis(150)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    if key_tx.send(AppEvent::Key(key)).is_err() {
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

    thread::spawn(move || loop {
        thread::sleep(TICK_INTERVAL);
        if tx.send(AppEvent::Tick).is_err() {
            return;
        }
    });

    let mut app = App::new();
    loop {
        terminal.draw(|frame| render(frame, &app))?;

        match rx.recv() {
            Ok(AppEvent::Client(ClientEvent::Unreachable(reason))) => app.set_unreachable(reason),
            Ok(AppEvent::Client(ClientEvent::Reconnecting)) => app.set_reconnecting(),
            Ok(AppEvent::Client(ClientEvent::Snapshot(agents, test_runs))) => {
                app.apply_snapshot(agents, test_runs)
            }
            Ok(AppEvent::Client(ClientEvent::Update(agent))) => app.apply_update(agent),
            Ok(AppEvent::Client(ClientEvent::TestRunUpdate(test_run))) => {
                app.apply_test_run_update(test_run)
            }
            Ok(AppEvent::Key(key)) => {
                if handle_key(key) == InputAction::Quit {
                    return Ok(());
                }
            }
            Ok(AppEvent::Tick) => {}
            Err(_) => return Ok(()),
        }
    }
}
