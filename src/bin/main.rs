extern crate mtaillib;
use mtaillib::mtail::MultiTail;
use std::io::{self, Write};

fn main() {
	let m = MultiTail::new(vec!["/Users/joel.self/Projects/joel/test.log".to_string(),
												 "/Users/joel.self/Projects/joel/test2.log".to_string()]);
	loop {
		let msgs = m.get_received();
		for (thread, s) in msgs {
			print!("{}:\t{}", thread, s);
			io::stdout().flush().unwrap();
		}
	}
}