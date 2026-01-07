#!/bin/sh
swift build -c release -Xswiftc -wmo
cargo build --release --manifest-path ./rust/Cargo.toml

cp ./.build/release/AWDLDisablerApp ./AWDLDisabler.app/Contents/MacOS/AWDLDisablerApp
cp ./rust/target/release/awdl-disabler ./AWDLDisabler.app/Contents/Library/LaunchDaemons/AWDLDisablerDaemon

chmod +x ./AWDLDisabler.app/Contents/MacOS/AWDLDisablerApp
chmod +x ./AWDLDisabler.app/Contents/Library/LaunchDaemons/AWDLDisablerDaemon

# codesign --force --deep --sign - AWDLDisabler.app