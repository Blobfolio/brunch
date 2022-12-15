/*!
# Brunch: Build
*/

fn main() {
	let ac = autocfg::new();
	ac.emit_path_cfg("std::hint::black_box", "no_brunch_black_box");
}
