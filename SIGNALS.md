# Supported Signals

killport supports the following signals via the `-s, --signal` flag. The default is `SIGKILL`.

## Gentle Signals

These signals can generally be caught and handled by processes for graceful shutdown.

| Signal | Description |
|---|---|
| `SIGTERM` | Termination signal (graceful shutdown) |
| `SIGINT` | Interrupt from keyboard (Ctrl+C) |
| `SIGHUP` | Hangup — terminal closed or parent process died |
| `SIGUSR1` | User-defined signal 1 |
| `SIGUSR2` | User-defined signal 2 |
| `SIGCONT` | Continue if stopped |
| `SIGALRM` | Timer signal |
| `SIGVTALRM` | Virtual timer expired |
| `SIGPROF` | Profiling timer expired |
| `SIGWINCH` | Window size change |
| `SIGURG` | Urgent condition on socket |
| `SIGCHLD` | Child status has changed |
| `SIGIO` | I/O now possible |
| `SIGPIPE` | Broken pipe (write to pipe with no readers) |
| `SIGPWR` | Power failure |
| `SIGSYS` | Bad argument to routine |

## Disruptive Signals

These signals are harder to handle and often terminate the process abruptly or produce a core dump.

| Signal | Description |
|---|---|
| `SIGQUIT` | Quit from keyboard and dump core |
| `SIGABRT` | Abort signal from `abort()` |
| `SIGTSTP` | Stop typed at terminal |
| `SIGTTIN` | Terminal input for background process |
| `SIGTTOU` | Terminal output for background process |
| `SIGSTOP` | Stop process (cannot be caught) |
| `SIGSEGV` | Invalid memory reference |
| `SIGBUS` | Bus error (bad memory access) |
| `SIGFPE` | Floating-point exception |
| `SIGILL` | Illegal instruction |
| `SIGTRAP` | Trace/breakpoint trap |

## Definitive Signals

Cannot be caught or ignored.

| Signal | Description |
|---|---|
| `SIGKILL` | Kill signal — immediate, unconditional termination |

## Usage

```sh
# Graceful termination
killport -s sigterm 8080

# Immediate kill (default)
killport -s sigkill 8080

# Interrupt
killport -s sigint 8080
```

> **Note**: Signal availability varies by platform. Not all signals are available on Windows.
