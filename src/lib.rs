//#![no_std]

//macro_rules! dyn_box {
	
//}
pub trait Fnord {
	fn foo(&self) -> u32;
}

struct A;
struct B(u128);

impl Fnord for A { fn foo(&self) -> u32 { 1 } }
impl Fnord for B { fn foo(&self) -> u32 { self.0 as u32 } }

#[repr(align(16))]
pub struct DynBox<const SIZE: usize> {
	store: [u8; SIZE],
	vtable: usize
}

impl <const SIZE: usize> DynBox<SIZE> {
	pub fn new() -> DynBox<SIZE> {
		DynBox {
			store: [0; SIZE],
			vtable: 0
		}
	}

	pub fn set<T: Fnord>(&mut self, content: T) {
		if !self.empty() {
			self.clear();
		}

		let size = core::mem::size_of::<T>();

		assert!(size <= SIZE);


		let parts: [usize; 2] = unsafe { core::mem::transmute(&content as *const dyn Fnord) };
		self.vtable = parts[1];
		println!("{:08x?}", parts);
		println!("writing {} bytes", size);
		unsafe { (&mut self.store as *mut _ as *mut T).copy_from(parts[0] as *mut _, 1); }
		println!("foo");
		core::mem::forget(content);
		println!("bar");
	}

	pub fn clear(&mut self) {
		if self.vtable != 0 {
			unsafe { core::ptr::drop_in_place(self.get_ptr_mut()) }
			self.vtable = 0;
		}
	}

	pub fn empty(&self) -> bool {
		self.vtable == 0
	}

	pub fn get(&self) -> Option<&dyn Fnord> {
		if self.vtable == 0 {
			None
		}
		else {
			Some(unsafe { &*self.get_ptr_mut() })
		}
	}

	pub fn get_mut(&mut self) -> Option<&mut dyn Fnord> {
		if self.vtable == 0 {
			None
		}
		else {
			Some(unsafe { &mut *self.get_ptr_mut() })
		}
	}

	unsafe fn get_ptr_mut(&self) -> *mut dyn Fnord {
		let foo: [usize; 2] = [ &self.store as *const _ as usize, self.vtable ];
		println!("get {:08x?}", foo);
		return core::mem::transmute(foo)
	}
}

mod tests {
	use super::*;
	#[test]
	fn new_dynbox_is_empty() {
		let mut dynbox = DynBox::<64>::new();
		assert!(dynbox.empty());
		assert!(dynbox.get().is_none());
		assert!(dynbox.get_mut().is_none());
	}

	#[test]
	fn set_dynbox_is_not_empty_and_clear_makes_it_empty() {
		let a = A;

		let mut dynbox = DynBox::<64>::new();
		dynbox.set(a);
		assert!(!dynbox.empty());

		dynbox.clear();
		assert!(dynbox.empty());
	}

	#[test]
	fn zero_sized() {
		let a = A;

		let mut dynbox = DynBox::<64>::new();
		assert!(dynbox.empty());
		assert!(dynbox.get().is_none());
		assert!(dynbox.get_mut().is_none());

		dynbox.set(a);
		assert!(!dynbox.empty());

		assert!(dynbox.get().unwrap().foo() == 1);
		assert!(dynbox.get_mut().unwrap().foo() == 1);
	}

	#[test]
	fn set_smaller() {
		let b = B(42);

		let mut dynbox = DynBox::<64>::new();
		dynbox.set(b);
		println!("baz");
		assert!(dynbox.get().unwrap().foo() == 42);
		assert!(dynbox.get_mut().unwrap().foo() == 42);

		dynbox.clear();
		println!("success");

	}

	#[test]
	fn set_same_size() {
		let b = B(42);

		let mut dynbox = DynBox::<16>::new();
		dynbox.set(b);
		assert!(dynbox.get().unwrap().foo() == 42);
		assert!(dynbox.get_mut().unwrap().foo() == 42);
	}

	#[test]
	#[should_panic]
	fn set_too_large_panics() {
		let b = B(42);

		let mut dynbox = DynBox::<4>::new();
		dynbox.set(b);
		assert!(dynbox.get().unwrap().foo() == 42);
		assert!(dynbox.get_mut().unwrap().foo() == 42);
	}
}
