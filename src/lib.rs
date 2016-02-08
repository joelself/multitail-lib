extern crate notify;
extern crate libc;
#[cfg(target_os = "macos")]
extern crate fsevent;
use std::boxed::Box;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::{Arc, Mutex, mpsc};
use std::{thread, str};
use std::thread::JoinHandle;
use std::fs;
use std::fs::File;
use std::io::SeekFrom;
use std::io::prelude::*;
//use std::ffi::CStr;
use libc::{c_char, size_t};
use std::cell::RefCell;
use notify::{PollWatcher, Error, Watcher, op};
#[macro_use]
mod macros;
#[cfg(not(target_os = "macos"))] pub type TailWatcher = RecommendedWatcher;
#[cfg(target_os = "macos")] pub type TailWatcher = PollWatcher;

const TX_BUF_SIZE: usize = 1024usize;

#[no_mangle]
pub extern "C" fn start_all_tails(array_file_path: *const *const c_char, length: size_t) -> Box<MultiTail> {
	let mut files = vec![];
	let string_array: &[&[u8]] = unsafe {
		std::slice::from_raw_parts(array_file_path as *const &[u8], length as usize)
	};
	for i in 0..length {
		files.push(str::from_utf8(string_array[i]).unwrap().to_string());
	}
	start_all_tails_internal(files)
}
pub fn start_all_tails_internal(files: Vec<String>) -> Box<MultiTail> {
	Box::new(MultiTail::new(files))
}

pub struct MultiTail {
	handles: RefCell<Vec<JoinHandle<()>>>,
	files: Vec<String>,
	global_buffer: Arc<Mutex<Vec<(usize, Vec<u8>)>>>,
	receiver: JoinHandle<()>,
}

struct TailBytes {
	thread: usize,
	bytes: RefCell<Vec<u8>>,
	last_nl: isize,
}
#[repr(C)]
pub struct Tuple {
    a: libc::uint32_t,
    b: libc::uint32_t,
}

impl MultiTail {
	pub fn new(files: Vec<String>) -> MultiTail {
		let (tx, rx) : (Sender<TailBytes>, Receiver<TailBytes>) = mpsc::channel();
		let thread_buffers: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(vec![]));
		let global_buffer: Arc<Mutex<Vec<(usize, Vec<u8>)>>> = Arc::new(Mutex::new(vec![]));
		let handles: RefCell<Vec<JoinHandle<()>>> = RefCell::new(vec![]);
		let mut thread_num = 0;
		for filepath in files.iter() {
			let filepath = filepath.clone();
			let tx_new = tx.clone();
			handles.borrow_mut().push(thread::spawn(move || {
				start_tail(thread_num, filepath, tx_new);
			}));
			let mut thread_buffers = thread_buffers.lock().unwrap();
			thread_buffers.push(vec![]);
			thread_num += 1;
		}
		let global_buf_ref = global_buffer.clone();
		let receiver = thread::spawn(move || {
			MultiTail::receive_msgs(rx, thread_buffers.clone(), global_buf_ref);
		});
		return MultiTail{handles: RefCell::new(vec![]), files: files.clone(), global_buffer: global_buffer.clone(),
										 receiver: receiver}
	}

	fn receive_msgs(rx: Receiver<TailBytes>, thread_buffers: Arc<Mutex<Vec<Vec<u8>>>>,
									global_buffer: Arc<Mutex<Vec<(usize, Vec<u8>)>>>,) {
		loop {
			match rx.recv() {
				Ok(bytes) => {
					if bytes.last_nl >= 0 {
						// got a printable message
						// lock thread buffers
						let mut thread_buffers = thread_buffers.lock().unwrap();
						thread_buffers[bytes.thread].extend_from_slice(&bytes.bytes.borrow()[..bytes.last_nl as usize]);
						// lock global buffer
						let mut global_buffer = global_buffer.lock().unwrap();
						let mut new_vec: Vec<u8> = vec![];
						new_vec.extend_from_slice(&bytes.bytes.borrow()[bytes.last_nl as usize..]);
						global_buffer.push((bytes.thread, thread_buffers.remove(bytes.thread))); // move the thread buffer to the global buffer
						thread_buffers.insert(bytes.thread, new_vec); // replace the thread buffer with a new one
					} else {
						// got a an unprintable message, append it to the current buffer
						// lock thread buffers
						let mut thread_buffers = thread_buffers.lock().unwrap();
						thread_buffers[bytes.thread].append(& mut *bytes.bytes.borrow_mut());
					}
				},
				_ => (),
			}
		}
	}

	pub fn get_received(&self) -> Vec<(usize, String)> {
		let mut global_buffer = self.global_buffer.lock().unwrap();
		let mut ret: Vec<(usize, String)> = vec![];
		ret.reserve(global_buffer.len());
		for i in 0..global_buffer.len() {
			let (thread, v) = global_buffer.remove(i);
			ret.push((thread, String::from_utf8(v).unwrap()));
		}
		ret
	}
}


fn open_and_seek<'a>(filepath: &str) -> File {
	// Output up to the last 2 newlines or 2048 bytes, whichever is less
	let mut file = File::open(filepath).unwrap();
	let mut size: u64 = fs::metadata(filepath).unwrap().len();
	if size > TX_BUF_SIZE as u64 {
		size = TX_BUF_SIZE as u64;
	}
	let mut bytes: Vec<u8> = vec![];
	let mut nls = 0;
	file.seek(SeekFrom::End(-(size as i64))).unwrap();
	let _unused = file.read_to_end(&mut bytes);
	for i in 0..bytes.len() - 1 {
		if bytes[size as usize -1 - i] == 0x0A {
			nls += 1;
			if nls == 2 {
				// Found the second newline, don't include it in the returned slice
				return file;
			}
		}
	}
	// Didn't find 2 newlines, just return TX_BUF_SIZE bytes from the end of the file
	return file;
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
	watcher: Option<TailWatcher>,
	tx: Sender<TailBytes>,
}

impl Channel {
	#[cfg(target_os = "macos")]
	pub fn new(tx: Sender<notify::Event>, filepath: String, tx_parent: Sender<TailBytes>)
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
		let w: Result<PollWatcher, Error> = PollWatcher::new(tx);
		let watcher = match w {
			Ok(mut watcher) => {
				let _unused = watcher.watch(&filepath);
				Some(watcher)
			}
			Err(_) 				=> None,
		};
		Channel{join_handle: None, watcher: watcher, tx: tx_parent}
	}

	#[cfg(any(target_os = "linux", target_os = "windows"))]
	pub fn new(tx: Sender<notify::Event>, filepath: String, tx_parent: Sender<TailBytes>) -> Channel {
		let mut w: Result<TailWatcher, Error> = TailWatcher::new(tx);
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

fn start_tail<'b>(thread: usize, filepath: String, tx_parent: Sender<TailBytes>) {
	let mut buffer: Vec<u8> = vec![];
	// Currently the notify library for Rust doesn't work with MacOS X FSEvents on Rust 1.6.0,
	// and MacOS 10.10.5, so there's two different config methods for setting up a channel
	let (tx, rx) = channel();
	let channel = Channel::new(tx, filepath.clone(), tx_parent);
	let mut file = open_and_seek(&filepath);
	loop {
		buffer.clear();
		match rx.recv() {
			Ok(event) => {
				if event.op.unwrap() == op::WRITE {
					// Read to eof
					let _bytes_read = file.read_to_end(& mut buffer).unwrap();
					// Get index of last newline
					for chunk in buffer.chunks(TX_BUF_SIZE) {
						let last_nl = find_last_nl_slice(chunk);
						channel.tx.send(TailBytes{thread: thread, bytes: RefCell::new(chunk.to_vec()), last_nl: last_nl}).unwrap();
					}
					// console.attr(attr);
					// TODO: actually handle the result
					// Seek back to just after the last nl
					let last_nl = find_last_nl(&buffer);
					file.seek(SeekFrom::Current(last_nl as i64 - buffer.len() as i64 - 1)).unwrap();
				}
			},
			_ => (),
		}
	}
}
