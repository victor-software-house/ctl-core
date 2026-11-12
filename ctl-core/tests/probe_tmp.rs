//! Probe.
#![cfg(feature = "input")]
use ctl_core::input::Input;

#[test]
fn probe() {
    for text in [
        "? [a, b]\n: v\nz: w\n",
        "? {c: d}\n: u\ny: x\n",
        "a: 1\n? [a]\n: v\nz: w\n",
    ] {
        let input = Input::new("p.yaml", text);
        println!("{text:?} => {:?}", input.top_level());
    }
}
