require "spec_helper"
require "socket"
require "json"
require "tmpdir"

RSpec.describe AgentMonitorRspecFormatter do
  # Spins up a real UNIXServer at a temporary path, points the formatter at
  # it via AGENTMON_SOCKET_PATH, and yields the one JSON message it receives
  # (or nil if none arrives within the timeout) - a real socket rather than a
  # mock, since the formatter's own value is entirely in what it puts on the
  # wire.
  def with_mock_daemon
    Dir.mktmpdir do |dir|
      socket_path = File.join(dir, "agentd.sock")
      server = UNIXServer.new(socket_path)
      received = nil
      accept_thread = Thread.new do
        client = server.accept
        received = client.gets
        client.close
      end

      previous = ENV["AGENTMON_SOCKET_PATH"]
      ENV["AGENTMON_SOCKET_PATH"] = socket_path
      begin
        yield
      ensure
        ENV["AGENTMON_SOCKET_PATH"] = previous
      end

      accept_thread.join(1)
      server.close
      received && JSON.parse(received)
    end
  end

  let(:formatter) { described_class.new(StringIO.new) }

  it "reports a started event when the run begins" do
    message = with_mock_daemon do
      formatter.start(RSpec::Core::Notifications::StartNotification.new(1, 0.0))
    end

    expect(message).to include("type" => "report_test_run", "status" => "started")
    expect(message["cwd"]).to eq(Dir.pwd)
    expect(message["pid"]).to eq(Process.pid)
  end

  it "reports a passed event when the summary has no failures" do
    summary = RSpec::Core::Notifications::SummaryNotification.new(0.01, [], [], [], 0.01, 0)

    message = with_mock_daemon { formatter.dump_summary(summary) }

    expect(message).to include("status" => "passed")
  end

  it "reports a failed event when the summary has at least one failure" do
    summary = RSpec::Core::Notifications::SummaryNotification.new(0.01, [], ["a failed example"], [], 0.01, 0)

    message = with_mock_daemon { formatter.dump_summary(summary) }

    expect(message).to include("status" => "failed")
  end

  it "does not raise when the daemon socket is missing" do
    Dir.mktmpdir do |dir|
      previous = ENV["AGENTMON_SOCKET_PATH"]
      ENV["AGENTMON_SOCKET_PATH"] = File.join(dir, "no-daemon-here.sock")

      begin
        expect {
          formatter.dump_summary(
            RSpec::Core::Notifications::SummaryNotification.new(0.01, [], [], [], 0.01, 0)
          )
        }.not_to raise_error
      ensure
        ENV["AGENTMON_SOCKET_PATH"] = previous
      end
    end
  end
end
