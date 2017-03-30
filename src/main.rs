
#![allow(non_upper_case_globals)]

/// Port of
/// Enigma Simulator by Henry Tieman
/// by Nicholas Cyprus (ngc0202)

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::env;

const NUM_ROTORS: usize = 5;

static REF_ROTOR: [u8; 26] = *b"YRUHQSLDPXNGOKMIEBFZCWVJAT";

const ROTOR: [[u8; 26]; NUM_ROTORS] = [
	[b'E', b'K', b'M', b'F', b'L', b'G', b'D', b'Q', b'V', b'Z', b'N', b'T', b'O', b'W', b'Y', b'H', b'X', b'U', b'S', b'P', b'A', b'I', b'B', b'R', b'C', b'J'],
	[b'A', b'J', b'D', b'K', b'S', b'I', b'R', b'U', b'X', b'B', b'L', b'H', b'W', b'T', b'M', b'C', b'Q', b'G', b'Z', b'N', b'P', b'Y', b'F', b'V', b'O', b'E'],
	[b'B', b'D', b'F', b'H', b'J', b'L', b'C', b'P', b'R', b'T', b'X', b'V', b'Z', b'N', b'Y', b'E', b'I', b'W', b'G', b'A', b'K', b'M', b'U', b'S', b'Q', b'O'],
	[b'E', b'S', b'O', b'V', b'P', b'Z', b'J', b'A', b'Y', b'Q', b'U', b'I', b'R', b'H', b'X', b'L', b'N', b'F', b'T', b'G', b'K', b'D', b'C', b'M', b'W', b'B'],
	[b'V', b'Z', b'B', b'R', b'G', b'I', b'T', b'Y', b'U', b'P', b'S', b'D', b'N', b'H', b'L', b'X', b'A', b'W', b'M', b'J', b'Q', b'O', b'F', b'E', b'C', b'K'],
];

/* steps at: q, e, v, j, z */
const step_data: [u8; NUM_ROTORS] = [
	b'q' - b'a',
	b'e' - b'a',
	b'v' - b'a',
	b'j' - b'a',
	b'z' - b'a',
];

#[derive(Debug, Clone)]
pub struct Enigma {
	order:		[usize; 3], 	// rotor order
	ring: 		[u8; 8],		// ring settings
	n_plugs:	usize,			// number of plugs
	plugs: 		[u8; 13],		// plug string
	pos: 		[u8; 3],		// rotor positions
	data:		[[u8; 26]; 8],	// working data for machine
	step: 		[u8; 3], 		// steps corresponding to motors
	dstep:		bool,			// double_step
}

fn usage() -> ! {
	let _ = writeln!(io::stderr(), "Usage: [keyfile] <infile>");
	std::process::exit(1);
}

macro_rules! usage {
    ($res:expr) => (
    	match $res {
    		Ok(v) => v,
    		Err(_) => usage(),
    	}
    )
}

macro_rules! usage_opt {
    ($opt:expr) => (
    	match $opt {
    		Some(v) => v,
    		None 	=> usage(),
    	}
    )
}

fn toupper(c: u8) -> u8 {
	if c >= b'a' && c <= b'z' {
		c - (b'a' - b'A')
	} else {
		c
	}
}

macro_rules! unwrap {
    ($res:expr) => (
    	match $res {
    		Ok(v) => v,
    		Err(e) => {
    			let _ = writeln!(io::stderr(), "{}", e);
				std::process::exit(1);
    		}
    	}
    )
}

impl Enigma {

	fn init(&mut self) {
		// set up rotor data
		for j in 0..26 {
			self.data[4][j] = (REF_ROTOR[j] + 26 - b'A') % 26;
		}

		for i in 1..4 {
			self.step[i-1] = step_data[self.order[i-1]];
			for j in 0..26 {
				self.data[i][j] = (ROTOR[self.order[i-1]][j] + 26 - b'A') % 26;
				self.data[8-i][self.data[i][j] as usize] = j as u8;
			}
		}

		// set up ring settings
		self.ring[7] = self.ring[1];
		self.ring[6] = self.ring[2];
		self.ring[5] = self.ring[3];
		for i in 1..8 {
			if i != 4 {
				let ds = self.ring[i] - b'A';
				if ds != 0 {
					for j in 0..26 {
						self.data[0][j] = self.data[i][j];
					}

					for j in 0..26 {
						self.data[i][j] = self.data[0][(52 - ds + j as u8) as usize % 26];
					}
				}
			}
		}

		// set up plug data
		if self.n_plugs != 0 {
			for i in 0..26 {
				self.data[0][i] = i as u8;
			}

			let mut j = 0;
			for _ in 0..self.n_plugs {
				while !is_alpha(self.plugs[j]) {
					j += 1;
					if self.plugs[j] == 0 {
						break;
					}
				}

				let u = toupper(self.plugs[j]) - b'A';
				j += 1;
				let v = toupper(self.plugs[j]) - b'A';
				j += 1;
				self.data[0][u as usize] = v;
				self.data[0][v as usize] = u;
			}
		}

		// convert all moving rotor data to displacements
		for i in 1..8 {
			if i != 4 {
				for j in 0..26 {
					self.data[i][j] = (26 + self.data[i][j] - j as u8) % 26;
				}
			}
		}
	}

	fn read_keyfile(&mut self, p: &str) -> io::Result<()> {
		let kf = File::open(p)?;
		let mut kf = BufReader::new(kf);

		let mut buf = String::new();
		kf.read_line(&mut buf)?;
		let mut count = 0;
		for i in buf.split_whitespace() {
			let i = usage!(i.parse());
			self.order[count] = i;
			count += 1;
		}

		if count < 3 {
			println!("Invalid keyfile.");
			usage();
		}

		kf.read_line(&mut buf)?;
		count = 0;
		for i in buf.split_whitespace() {
			let i = usage!(i.parse());
			self.ring[count+1] = i;
			count += 1;
		}

		if count < 3 {
			println!("Invalid keyfile.");
			usage();
		}

		kf.read_line(&mut buf)?;
		self.n_plugs = usage!(
			usage_opt!(
				buf.split_whitespace().next()
			).parse()
		);

		if self.n_plugs > 0 {
			kf.read_line(&mut buf)?;
			for (c, i) in buf.split_whitespace().enumerate() {
				self.plugs[c] = usage!(i.parse());
			}
		}

		// dummy for input
		let mut a = [0u8; 3];
		count = 0;
		kf.read_line(&mut buf)?;
		for i in buf.split_whitespace() {
			if count >= a.len() { break; }
			a[count] = usage!(i.parse());
			count += 1;
		}

		for idx in 0..3 {
			self.order[idx] -= 1;
			self.ring[idx+1] = toupper(self.ring[idx+1]);
			self.pos[idx] = toupper(a[idx]) - b'A';
		}

		Ok(())
	}

	fn advance_rotors(&mut self) {
		self.pos[0] = (self.pos[0] + 1) % 26;

		if self.pos[0] == self.step[0] {
			self.pos[1] = (self.pos[1] + 1) % 26;
		}

		self.pos[1] = (self.pos[1] + 1) % 26;

		if self.dstep {
			self.pos[1] = (self.pos[1] + 1) % 26;
			self.pos[2] = (self.pos[2] + 1) % 26;
			self.dstep = false;
		}

		if self.pos[1] == self.step[1] {
			self.dstep = true;
		}
	}

    fn encipher(&mut self, mut c: u8) -> u8 {
        let mut idx;	// index for counting

        if is_alpha(c) {
            self.advance_rotors();

            // start to enciper
            c = toupper(c);
            c -= b'A';

            if self.n_plugs != 0 {
                c = self.data[0][c as usize];
            }

            // do rotors forward
            for j in 0..3 {
                idx = (c + self.pos[j]) % 26;
                c 	= (c + self.data[j+1][idx as usize]) % 26;
            }

            // reflecting rotor
            c = (self.data[4][c as usize]) % 26;

            // do rotors reverse 
            for j in 0..3 {
                idx = (c + self.pos[2-j]) % 26;
                c 	= (c + self.data[j+5][idx as usize]) % 26;
            }

            if self.n_plugs != 0 {
                c = self.data[0][c as usize];
            }

            c += b'A';
            c
        } else {
            c
        }
    }
}


fn is_alpha(c: u8) -> bool {
	(c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z')
}



fn encipher_file(path: &str, en: &mut Enigma) -> io::Result<()> {
	let f = BufReader::new(File::open(path)?);
	for b in f.bytes() {
		print!("{}", en.encipher(b?) as char);
	}

    println!();

	Ok(())
}

fn main() {
	let args: Vec<_> = env::args().collect();

    let mut en = Enigma {
        order:      [0, 1, 2],
        ring:       *b"\0AAA\0\0\0\0",
        n_plugs:    0,
        plugs:      [0u8; 13],
        pos:        [0, 0, 0],
        data:       [[0u8; 26]; 8],
        step:       [0u8; 3],
        dstep:      false,
    };


    match args.len() {
		3 => {
			en.init();
			unwrap!(en.read_keyfile(&args[1]));
			unwrap!(encipher_file(&args[2], &mut en));
		}

		2 => {
			en.init();
			unwrap!(encipher_file(&args[1], &mut en));
		}

		_ => usage(),
	}
}
