use std::fmt;

macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub struct $name(pub(crate) u32);

        impl $name {
            pub fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            pub fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_handle!(PointHandle);
define_handle!(VertexHandle);
define_handle!(PrimHandle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_index_roundtrip() {
        let h = PointHandle::from_index(42);
        assert_eq!(h.index(), 42);

        let h = VertexHandle::from_index(0);
        assert_eq!(h.index(), 0);

        let h = PrimHandle::from_index(999);
        assert_eq!(h.index(), 999);
    }

    #[test]
    fn equality() {
        assert_eq!(PointHandle::from_index(1), PointHandle::from_index(1));
        assert_ne!(PointHandle::from_index(1), PointHandle::from_index(2));
    }

    #[test]
    fn ordering() {
        assert!(PointHandle::from_index(0) < PointHandle::from_index(1));
        assert!(VertexHandle::from_index(5) > VertexHandle::from_index(3));
    }

    #[test]
    fn debug_format() {
        let h = PointHandle::from_index(7);
        assert_eq!(format!("{:?}", h), "PointHandle(7)");
    }

    #[test]
    fn display_format() {
        let h = PrimHandle::from_index(3);
        assert_eq!(format!("{}", h), "PrimHandle(3)");
    }
}
