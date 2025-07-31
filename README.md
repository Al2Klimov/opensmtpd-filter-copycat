## About

OpenSMTPd filter which rejects eMails from `A_NAME <A_LOCAL@A_HOST>` to
`B_NAME <B_LOCAL@B_HOST>` if `A_NAME` contains `B_HOST` (mostly spam).

## Build

Compile like any other Rust program: `cargo build -r`

Find the resulting binary directly under `target/release/`.

## Usage

Integrate this filter into smtpd.conf(5).
