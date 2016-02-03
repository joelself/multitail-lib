extern crate notify;
extern crate libc;
#[cfg(target_os = "macos")]
extern crate fsevent;
use std::boxed::Box;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::{thread, env, str};
use std::thread::JoinHandle;
use std::fs;
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
use std::ffi::CStr;
use libc::c_char;
use notify::{RecommendedWatcher, PollWatcher, Error, Watcher, op};
use self::macros::*;
#[macro_use]
mod macros;

const TX_BUF_SIZE: usize = 1024usize;

#[no_mangle]
pub extern "C" fn start_all_talis(array_file_path: **const c_char, size_t length) -> Box<MultiTail> {
	let files = vec![];
	for i in 0..length {
		let array &[u8] = unsafe {
			std::slice::from_raw_parts(array_pointer as *const u8, size as usize) 
		}
		files.push(String::from_utf8(sparkle_heart).unwrap());
	}
	start_all_tails_internal(files)
}
fn pub start_all_tails_internal(files: Vec<String>) -> Box<MultiTail> {
	let mtail = MultiTail::new(files);
}

struct MultiTail {
	handles: RefCell<vec<JoinHandle<()>>>;
	files: Vec<String> files;

}

struct TailBytes {
	thread: usize;
	bytes: &[u8];
	last_nl: isize;
}

impl MultiTail {
	pub fn new(files: Vec<String>) -> MultiTail {
		let res = MultiTail{handles: vec![], files: files.clone()};
		let (tx, rx) = channel<TailBytes>();
		let thread_buffers = Vec<Vec<u8>> = vec![];
		let global_buffer = Vec<u8> = vec![];
		let thread_num = 0;
		for filepath in matches.iter() {
			let filepath = filepath.clone();
			res.handles.borrow_mut().push(thread::spawn(move || {
				start_tail(thread_num, filepath, tx.clone());
			}));
			buffers.push(vec![]);
			thread_num += 1;
		}

		loop {
			match rx.recv() {
				Ok(bytes) => {
					if(bytes.last_nl >= 0) {
						// lock thread buffers
						thread_buffers[bytes.thread].extend_from_slice(&bytes.bytes[..bytes.last_nl]);
						// lock global buffer
						global_buffer.push(thread_buffers.remove(bytes.thread));
						thread_buffers.insert(bytes.thread, vec![]
							.extend_from_size(&bytes.bytes[bytes.last_nl..]));
					} else {
						// lock thread buffers
						thread_buffers[bytes.thread].extend_from_slice(bytes.bytes);
					}
				},
				_ => (),
			}
		}

		while handles.len() > 0 {
			let handle = handles.pop();
			if let Some(h) = handle {
				h.join();
			}
		}
	}
}

fn open_and_seek<'a>(filepath: &str, buf: &'a mut [u8;2048]) -> (File, &'a [u8]) {
	// Output up to the last 2 newlines or 2048 bytes, whichever is less
	const GET_BYTES: u64 = 2048u64;
	let mut file = File::open(filepath).unwrap();
	let mut size: u64 = fs::metadata(filepath).unwrap().len();
	if size > GET_BYTES {
		size = GET_BYTES;
	}
	let mut bytes: Vec<u8> = vec![];
	let nls = 0;
	file.seek(SeekFrom::End(-(size as i64))).unwrap();
	file.read_to_end(&mut bytes);
	for i in 0..bytes.len() - 1 {
		if bytes[size as usize -1 - i] == 0x0A {
			nls += 1;
			if nls == 2 {
				// Found the second newline, don't include it in the returned slice
				return (file, &buf[..i]);
			}
		}
		buf[i] = bytes[i];
	}
	// Didn't find 2 newlines, just return 2048 bytes of data
	return (file, &buf[..]);
}

fn find_last_nl(buf: &Vec<u8>) -> usize {
	let iter = buf.iter().rev();
	let len = buf.len();
	for i in 0..len {
		if buf[len - 1 - i] == 0x0A {
			if i+1 < len && buf[len - 2 - i] == 0x0D {
				return len - 2 - i;
			} else {
				return len - 1 - i;
			}
		}
	}
	return buf.len();
}

fn find_last_nl_slice(buf: &[u8]) -> isize {
	let iter = buf.iter().rev();
	let len = buf.len();
	for i in 0..len {
		if buf[len - 1 - i] == 0x0A {
			if i+1 < len && buf[len - 2 - i] == 0x0D {
				return (len - 2 - i) as isize;
			} else {
				return (len - 1 - i) as isize;
			}
		}
	}
	return -1;
}



struct Channel {
	join_handle: Option<JoinHandle<()>>,
	watcher: Option<RecommendedWatcher>,
	tx: Sender<notify::Event>,
}

impl Channel {
	#[cfg(target_os = "macos")]
	pub fn new(tx: Sender<fsevent::Event>, filepath: String, tx_parent: Sender<TailBytes>)
	-> Channel {
		// let fp = filepath.clone();
		// let jh: JoinHandle<()> = thread::spawn(move || {
		//    let fsevent = fsevent::FsEvent::new(tx);
		//    fsevent.append_path(&filepath);
		//    fsevent.observe();
		//  });
		// lock_wr_fl!(console, "Got observer for file: \"{}\"", fp);
		// Channel{join_handle: Some(jh), watcher: None}
		// You can't watch some files (a lot of the files you would want to tail) using FSEvents
		// So I'm just going to default to the polling watcher on MacOS
		let mut w: Result<PollWatcher, Error> = PollWatcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => Some(watcher),
			Err(err) 				=> None,
		};
		Channel{join_handle: None, watcher: watcher, tx: tx_parent}
	}

	#[cfg(any(target_os = "linux", target_os = "windows"))]
	pub fn new(tx: Sender<notify::Event>, filepath: String, tx_parent: Sender<TailBytes>) -> Channel {
		let mut w: Result<RecommendedWatcher, Error> = RecommendedWatcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => {
				watcher.watch(&filepath);
				Some(watcher)
			},
			Err(err) 				=> None,
		};
		Channel{join_handle: None, watcher: watcher, tx: tx_parent}
	}
}

fn start_tail(thread: usize, filepath: String, tx_parent: Sender<TailBytes>) {
	// Currently the notify library for Rust doesn't work with MacOS X FSEvents on Rust 1.6.0,
	// and MacOS 10.10.5, so there's two different config methods for setting up a channel
	let (tx, rx) = channel();	
	let channel = Channel::new(tx, filepath.clone(), tx_parent);
	let mut buf: [u8;TX_BUF_SIZE] = [0; TX_BUF_SIZE];
	let (mut file, buf_slice) = open_and_seek(&filepath, &mut buf);
	lock_wr_fl!(console: fg_color: bg_color, "\n{}", str::from_utf8(buf_slice).unwrap());
	loop {
		match rx.recv() {
			Ok(event) => {
				if event.op.unwrap() == op::WRITE {
					let mut buf: Vec<u8> = vec![];
					let bytes_read: usize = 1;
					// Read to eof
					bytes_read = file.read_to_end(& mut buf).unwrap();
					// Get index of last newline
					let pos = 0;
					for chunk in buf.chunks(TX_BUF_SIZE) {
						let last_nl = find_last_nl_slice(chunk);
						channel.tx.send(TailBytes{thread: thread, bytes: chunk, last_nl: last_nl}).unwrap();
					}
					// console.attr(attr);
					// TODO: actually handle the result
					// Seek back to just after the last nl
					file.seek(SeekFrom::Current(last_nl as i64 - buf.len() as i64 - 1)).unwrap();
					}
				}
			},
			_ => (),
		}
	}
}

