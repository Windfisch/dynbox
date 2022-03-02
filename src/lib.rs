#![no_std]

#[allow(unused_macros)]
#[macro_export]
/// Generates a DynBox-like data structure that can hold a certain trait.
/// Example:
/// ```
/// use dyn_box::dynbox;
/// trait MyTrait { fn foo(&self) -> u32; }
/// dynbox!(MyDynBox: MyTrait);
/// let my_dynbox = MyDynBox::<16>::new();
/// ```
macro_rules! dynbox {
	($name:ident : $trait:ident) => {
		#[repr(align(16))]
		/// DynBox for a given Trait with `Option<dyn Trait>`-like semantics.
		/// Can hold implementors of the trait if they do not exceed `SIZE`, and
		/// allows to retrieve `&dyn Trait` and `&mut dyn Trait` references.
		/// Generated through the `dynbox!` macro
		pub struct $name<const SIZE: usize> {
			store: [u8; SIZE],
			vtable: usize,
		}

		impl<const SIZE: usize> Drop for $name<SIZE> {
			fn drop(&mut self) {
				self.clear();
			}
		}

		impl<const SIZE: usize> $name<SIZE> {
			/// Creates a new empty DynBox.
			pub fn new() -> $name<SIZE> {
				$name {
					store: [0; SIZE],
					vtable: 0,
				}
			}

			/// Stores a value of some generic type which implements $trait. Panics if
			/// T's size exceeds `SIZE`. Clears (and drops) the previous value, if
			/// present.
			pub fn set<T: $trait>(&mut self, content: T) {
				if !self.empty() {
					self.clear();
				}

				let size = core::mem::size_of::<T>();

				assert!(size <= SIZE);

				let parts: [usize; 2] =
					unsafe { core::mem::transmute(&content as *const dyn $trait) };
				self.vtable = parts[1];
				unsafe {
					(&mut self.store as *mut _ as *mut T).copy_from(parts[0] as *mut _, 1);
				}
				core::mem::forget(content);
			}

			/// Makes the DynBox empty again by dropping the previous content, if any.
			pub fn clear(&mut self) {
				if self.vtable != 0 {
					unsafe { core::ptr::drop_in_place(self.get_ptr_mut()) }
					self.vtable = 0;
				}
			}

			/// Returns whether the DynBox currently contains any value.
			pub fn empty(&self) -> bool {
				self.vtable == 0
			}

			/// Returns a `&dyn Trait` reference if not empty, or None otherwise.
			pub fn get(&self) -> Option<&dyn $trait> {
				if self.vtable == 0 {
					None
				} else {
					Some(unsafe { &*self.get_ptr_mut() })
				}
			}

			/// Returns a `&mut dyn Trait` reference if not empty, or None otherwise.
			pub fn get_mut(&mut self) -> Option<&mut dyn $trait> {
				if self.vtable == 0 {
					None
				} else {
					Some(unsafe { &mut *self.get_ptr_mut() })
				}
			}

			unsafe fn get_ptr_mut(&self) -> *mut dyn $trait {
				let foo: [usize; 2] = [&self.store as *const _ as usize, self.vtable];
				return core::mem::transmute(foo);
			}
		}
	};
}

#[cfg(doc)]
/// Some object-safe trait
pub trait MyTrait {}

#[cfg(doc)]
dynbox!(MyDynBox: MyTrait);

#[cfg(test)]
mod tests {
	use core::cell::Cell;

	pub trait MyTrait {
		fn foo(&self) -> u32;
	}

	struct A;
	struct B(u128);
	struct Droppable<'a>(&'a Cell<bool>);

	impl MyTrait for A {
		fn foo(&self) -> u32 {
			1
		}
	}
	impl MyTrait for B {
		fn foo(&self) -> u32 {
			self.0 as u32
		}
	}
	impl MyTrait for Droppable<'_> {
		fn foo(&self) -> u32 {
			2
		}
	}
	impl Drop for Droppable<'_> {
		fn drop(&mut self) {
			self.0.set(true);
		}
	}

	dynbox!(DynBox: MyTrait);

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
		assert!(dynbox.get().unwrap().foo() == 42);
		assert!(dynbox.get_mut().unwrap().foo() == 42);

		dynbox.clear();
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

	#[test]
	fn drop_is_called_on_clear() {
		let drop_was_called = Cell::new(false);
		let d = Droppable(&drop_was_called);
		let mut dynbox = DynBox::<64>::new();

		dynbox.set(d);
		assert!(!drop_was_called.get());

		dynbox.clear();
		assert!(drop_was_called.get());
	}

	#[test]
	fn drop_is_called_on_set() {
		let drop_was_called = Cell::new(false);
		let d = Droppable(&drop_was_called);
		let a = A;
		let mut dynbox = DynBox::<64>::new();

		dynbox.set(d);
		assert!(!drop_was_called.get());

		dynbox.set(a);
		assert!(drop_was_called.get());
	}

	#[test]
	fn drop_is_called_on_drop() {
		let drop_was_called = Cell::new(false);
		{
			let d = Droppable(&drop_was_called);
			let mut dynbox = DynBox::<64>::new();

			dynbox.set(d);
			assert!(!drop_was_called.get());
		}

		assert!(drop_was_called.get());
	}
}
