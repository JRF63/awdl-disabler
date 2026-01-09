# AWDL Disabler

Utility for disabling awdl0. Inspired by [awdlkiller](https://github.com/jamestut/awdlkiller).

## CLI

Build the daemon with

```sh
cd rust
cargo build --release
```

Then run it with root privileges
```sh
# Optionally copy to /usr/local/bin
cp target/release/awdl-disabler /usr/local/bin/

# This needs root
sudo awdl-disabler
```

## Dock-only app

Run `build.sh`. This will build the Swift launcher and the Rust daemon.

Copy the AWDLDisabler app to Applications then run it from there.

Running the app will start the daemon and closing the app will stop it.

## Logging

View any errors in the app/daemon with the following command

```sh
log show --predicate 'subsystem == "awdldisabler.app"' --last 5m
```
