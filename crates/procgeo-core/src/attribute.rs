use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

// ---------------------------------------------------------------------------
// AttribClass
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttribClass {
    Point,
    Vertex,
    Primitive,
    Detail,
}

// ---------------------------------------------------------------------------
// AttribType
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttribType {
    Int,
    Int64,
    Float,
    Float64,
    Vector2,
    Vector3,
    Vector4,
    Matrix3,
    Matrix4,
    String,
}

// ---------------------------------------------------------------------------
// TypeQualifier
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeQualifier {
    None,
    Point,
    Vector,
    Normal,
    Color,
    Quaternion,
    Matrix,
}

impl Default for TypeQualifier {
    fn default() -> Self {
        TypeQualifier::None
    }
}

// ---------------------------------------------------------------------------
// AttribStorage — typed Vec<T> for each variant
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum AttribStorage {
    Int(Vec<i32>),
    Int64(Vec<i64>),
    Float(Vec<f32>),
    Float64(Vec<f64>),
    Vector2(Vec<[f32; 2]>),
    Vector3(Vec<[f32; 3]>),
    Vector4(Vec<[f32; 4]>),
    Matrix3(Vec<[f32; 9]>),
    Matrix4(Vec<[f32; 16]>),
    String(Vec<std::string::String>),
}

impl AttribStorage {
    pub fn len(&self) -> usize {
        match self {
            AttribStorage::Int(v) => v.len(),
            AttribStorage::Int64(v) => v.len(),
            AttribStorage::Float(v) => v.len(),
            AttribStorage::Float64(v) => v.len(),
            AttribStorage::Vector2(v) => v.len(),
            AttribStorage::Vector3(v) => v.len(),
            AttribStorage::Vector4(v) => v.len(),
            AttribStorage::Matrix3(v) => v.len(),
            AttribStorage::Matrix4(v) => v.len(),
            AttribStorage::String(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn attrib_type(&self) -> AttribType {
        match self {
            AttribStorage::Int(_) => AttribType::Int,
            AttribStorage::Int64(_) => AttribType::Int64,
            AttribStorage::Float(_) => AttribType::Float,
            AttribStorage::Float64(_) => AttribType::Float64,
            AttribStorage::Vector2(_) => AttribType::Vector2,
            AttribStorage::Vector3(_) => AttribType::Vector3,
            AttribStorage::Vector4(_) => AttribType::Vector4,
            AttribStorage::Matrix3(_) => AttribType::Matrix3,
            AttribStorage::Matrix4(_) => AttribType::Matrix4,
            AttribStorage::String(_) => AttribType::String,
        }
    }

    /// Resize to `new_len`, filling new slots with `default`.
    pub fn resize_with_default(&mut self, new_len: usize, default: &AttribDefault) {
        match (self, default) {
            (AttribStorage::Int(v), AttribDefault::Int(d)) => v.resize(new_len, *d),
            (AttribStorage::Int64(v), AttribDefault::Int64(d)) => v.resize(new_len, *d),
            (AttribStorage::Float(v), AttribDefault::Float(d)) => v.resize(new_len, *d),
            (AttribStorage::Float64(v), AttribDefault::Float64(d)) => v.resize(new_len, *d),
            (AttribStorage::Vector2(v), AttribDefault::Vector2(d)) => v.resize(new_len, *d),
            (AttribStorage::Vector3(v), AttribDefault::Vector3(d)) => v.resize(new_len, *d),
            (AttribStorage::Vector4(v), AttribDefault::Vector4(d)) => v.resize(new_len, *d),
            (AttribStorage::Matrix3(v), AttribDefault::Matrix3(d)) => v.resize(new_len, *d),
            (AttribStorage::Matrix4(v), AttribDefault::Matrix4(d)) => v.resize(new_len, *d),
            (AttribStorage::String(v), AttribDefault::String(d)) => v.resize(new_len, d.clone()),
            _ => panic!("resize_with_default: type mismatch between storage and default"),
        }
    }
}

// ---------------------------------------------------------------------------
// AttribDefault — single value per type
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum AttribDefault {
    Int(i32),
    Int64(i64),
    Float(f32),
    Float64(f64),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    Matrix3([f32; 9]),
    Matrix4([f32; 16]),
    String(std::string::String),
}

impl AttribDefault {
    pub fn attrib_type(&self) -> AttribType {
        match self {
            AttribDefault::Int(_) => AttribType::Int,
            AttribDefault::Int64(_) => AttribType::Int64,
            AttribDefault::Float(_) => AttribType::Float,
            AttribDefault::Float64(_) => AttribType::Float64,
            AttribDefault::Vector2(_) => AttribType::Vector2,
            AttribDefault::Vector3(_) => AttribType::Vector3,
            AttribDefault::Vector4(_) => AttribType::Vector4,
            AttribDefault::Matrix3(_) => AttribType::Matrix3,
            AttribDefault::Matrix4(_) => AttribType::Matrix4,
            AttribDefault::String(_) => AttribType::String,
        }
    }

    /// Create an empty (zero-length) storage of the matching type.
    pub fn empty_storage(&self) -> AttribStorage {
        match self {
            AttribDefault::Int(_) => AttribStorage::Int(Vec::new()),
            AttribDefault::Int64(_) => AttribStorage::Int64(Vec::new()),
            AttribDefault::Float(_) => AttribStorage::Float(Vec::new()),
            AttribDefault::Float64(_) => AttribStorage::Float64(Vec::new()),
            AttribDefault::Vector2(_) => AttribStorage::Vector2(Vec::new()),
            AttribDefault::Vector3(_) => AttribStorage::Vector3(Vec::new()),
            AttribDefault::Vector4(_) => AttribStorage::Vector4(Vec::new()),
            AttribDefault::Matrix3(_) => AttribStorage::Matrix3(Vec::new()),
            AttribDefault::Matrix4(_) => AttribStorage::Matrix4(Vec::new()),
            AttribDefault::String(_) => AttribStorage::String(Vec::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Attribute struct
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: std::string::String,
    pub class: AttribClass,
    pub qualifier: TypeQualifier,
    pub default: AttribDefault,
    pub storage: AttribStorage,
}

impl Attribute {
    pub fn new(
        name: impl Into<std::string::String>,
        class: AttribClass,
        default: AttribDefault,
        qualifier: TypeQualifier,
    ) -> Self {
        let storage = default.empty_storage();
        Attribute {
            name: name.into(),
            class,
            qualifier,
            default,
            storage,
        }
    }
}

// ---------------------------------------------------------------------------
// AttribHandle<T> — typed handle referencing an attribute by class + name
// ---------------------------------------------------------------------------

pub struct AttribHandle<T> {
    pub class: AttribClass,
    pub name: std::string::String,
    _marker: PhantomData<T>,
}

impl<T> AttribHandle<T> {
    pub fn new(class: AttribClass, name: impl Into<std::string::String>) -> Self {
        AttribHandle {
            class,
            name: name.into(),
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for AttribHandle<T> {
    fn clone(&self) -> Self {
        AttribHandle {
            class: self.class,
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for AttribHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttribHandle<{}>({:?}, {:?})",
            std::any::type_name::<T>(),
            self.class,
            self.name
        )
    }
}

// ---------------------------------------------------------------------------
// AttribValue trait + macro impls
// ---------------------------------------------------------------------------

pub trait AttribValue: Clone + Sized {
    fn attrib_type() -> AttribType;
    fn default_value() -> Self;

    fn get_from_storage(storage: &AttribStorage, index: usize) -> Result<Self, CoreError>;
    fn get_from_storage_mut(
        storage: &mut AttribStorage,
        index: usize,
    ) -> Result<&mut Self, CoreError>;
    fn set_in_storage(
        storage: &mut AttribStorage,
        index: usize,
        value: Self,
    ) -> Result<(), CoreError>;
    fn get_slice(storage: &AttribStorage) -> Result<&[Self], CoreError>;
    fn get_slice_mut(storage: &mut AttribStorage) -> Result<&mut [Self], CoreError>;
}

macro_rules! impl_attrib_value {
    ($rust_ty:ty, $variant:ident, $attrib_type:expr, $default_expr:expr) => {
        impl AttribValue for $rust_ty {
            fn attrib_type() -> AttribType {
                $attrib_type
            }

            fn default_value() -> Self {
                $default_expr
            }

            fn get_from_storage(
                storage: &AttribStorage,
                index: usize,
            ) -> Result<Self, CoreError> {
                if let AttribStorage::$variant(v) = storage {
                    Ok(v[index].clone())
                } else {
                    Err(CoreError::AttributeTypeMismatch(format!(
                        "expected {:?}, got {:?}",
                        $attrib_type,
                        storage.attrib_type()
                    )))
                }
            }

            fn get_from_storage_mut(
                storage: &mut AttribStorage,
                index: usize,
            ) -> Result<&mut Self, CoreError> {
                if let AttribStorage::$variant(v) = storage {
                    Ok(&mut v[index])
                } else {
                    Err(CoreError::AttributeTypeMismatch(format!(
                        "expected {:?}, got {:?}",
                        $attrib_type,
                        storage.attrib_type()
                    )))
                }
            }

            fn set_in_storage(
                storage: &mut AttribStorage,
                index: usize,
                value: Self,
            ) -> Result<(), CoreError> {
                if let AttribStorage::$variant(v) = storage {
                    v[index] = value;
                    Ok(())
                } else {
                    Err(CoreError::AttributeTypeMismatch(format!(
                        "expected {:?}, got {:?}",
                        $attrib_type,
                        storage.attrib_type()
                    )))
                }
            }

            fn get_slice(storage: &AttribStorage) -> Result<&[Self], CoreError> {
                if let AttribStorage::$variant(v) = storage {
                    Ok(v.as_slice())
                } else {
                    Err(CoreError::AttributeTypeMismatch(format!(
                        "expected {:?}, got {:?}",
                        $attrib_type,
                        storage.attrib_type()
                    )))
                }
            }

            fn get_slice_mut(storage: &mut AttribStorage) -> Result<&mut [Self], CoreError> {
                if let AttribStorage::$variant(v) = storage {
                    Ok(v.as_mut_slice())
                } else {
                    Err(CoreError::AttributeTypeMismatch(format!(
                        "expected {:?}, got {:?}",
                        $attrib_type,
                        storage.attrib_type()
                    )))
                }
            }
        }
    };
}

impl_attrib_value!(i32, Int, AttribType::Int, 0i32);
impl_attrib_value!(i64, Int64, AttribType::Int64, 0i64);
impl_attrib_value!(f32, Float, AttribType::Float, 0.0f32);
impl_attrib_value!(f64, Float64, AttribType::Float64, 0.0f64);
impl_attrib_value!([f32; 2], Vector2, AttribType::Vector2, [0.0f32; 2]);
impl_attrib_value!([f32; 3], Vector3, AttribType::Vector3, [0.0f32; 3]);
impl_attrib_value!([f32; 4], Vector4, AttribType::Vector4, [0.0f32; 4]);
impl_attrib_value!([f32; 9], Matrix3, AttribType::Matrix3, [0.0f32; 9]);
impl_attrib_value!([f32; 16], Matrix4, AttribType::Matrix4, [0.0f32; 16]);

// Manual impl for String
impl AttribValue for std::string::String {
    fn attrib_type() -> AttribType {
        AttribType::String
    }

    fn default_value() -> Self {
        std::string::String::new()
    }

    fn get_from_storage(storage: &AttribStorage, index: usize) -> Result<Self, CoreError> {
        if let AttribStorage::String(v) = storage {
            Ok(v[index].clone())
        } else {
            Err(CoreError::AttributeTypeMismatch(format!(
                "expected String, got {:?}",
                storage.attrib_type()
            )))
        }
    }

    fn get_from_storage_mut(
        storage: &mut AttribStorage,
        index: usize,
    ) -> Result<&mut Self, CoreError> {
        if let AttribStorage::String(v) = storage {
            Ok(&mut v[index])
        } else {
            Err(CoreError::AttributeTypeMismatch(format!(
                "expected String, got {:?}",
                storage.attrib_type()
            )))
        }
    }

    fn set_in_storage(
        storage: &mut AttribStorage,
        index: usize,
        value: Self,
    ) -> Result<(), CoreError> {
        if let AttribStorage::String(v) = storage {
            v[index] = value;
            Ok(())
        } else {
            Err(CoreError::AttributeTypeMismatch(format!(
                "expected String, got {:?}",
                storage.attrib_type()
            )))
        }
    }

    fn get_slice(storage: &AttribStorage) -> Result<&[Self], CoreError> {
        if let AttribStorage::String(v) = storage {
            Ok(v.as_slice())
        } else {
            Err(CoreError::AttributeTypeMismatch(format!(
                "expected String, got {:?}",
                storage.attrib_type()
            )))
        }
    }

    fn get_slice_mut(storage: &mut AttribStorage) -> Result<&mut [Self], CoreError> {
        if let AttribStorage::String(v) = storage {
            Ok(v.as_mut_slice())
        } else {
            Err(CoreError::AttributeTypeMismatch(format!(
                "expected String, got {:?}",
                storage.attrib_type()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// AttributeMap
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct AttributeMap {
    map: HashMap<(AttribClass, std::string::String), Attribute>,
}

impl AttributeMap {
    pub fn new() -> Self {
        AttributeMap {
            map: HashMap::new(),
        }
    }

    /// Create a new attribute. Returns error if an attribute with the same
    /// class + name already exists.
    pub fn create(
        &mut self,
        class: AttribClass,
        name: impl Into<std::string::String>,
        default: AttribDefault,
        qualifier: TypeQualifier,
    ) -> Result<(), CoreError> {
        let name = name.into();
        let key = (class, name.clone());
        if self.map.contains_key(&key) {
            return Err(CoreError::AttributeTypeMismatch(format!(
                "attribute already exists: {name}"
            )));
        }
        self.map
            .insert(key, Attribute::new(name, class, default, qualifier));
        Ok(())
    }

    /// Return a typed handle for an attribute. Validates that the type matches.
    pub fn find<T: AttribValue>(
        &self,
        class: AttribClass,
        name: impl AsRef<str>,
    ) -> Result<AttribHandle<T>, CoreError> {
        let name = name.as_ref();
        let key = (class, name.to_string());
        let attr = self
            .map
            .get(&key)
            .ok_or_else(|| CoreError::AttributeNotFound(name.to_string()))?;

        if attr.storage.attrib_type() != T::attrib_type() {
            return Err(CoreError::AttributeTypeMismatch(format!(
                "attribute '{}' has type {:?}, not {:?}",
                name,
                attr.storage.attrib_type(),
                T::attrib_type()
            )));
        }

        Ok(AttribHandle::new(class, name))
    }

    /// Get the value at `index` for the attribute referenced by `handle`.
    pub fn get<T: AttribValue>(
        &self,
        handle: &AttribHandle<T>,
        index: usize,
    ) -> Result<T, CoreError> {
        let key = (handle.class, handle.name.clone());
        let attr = self
            .map
            .get(&key)
            .ok_or_else(|| CoreError::AttributeNotFound(handle.name.clone()))?;
        T::get_from_storage(&attr.storage, index)
    }

    /// Set the value at `index` for the attribute referenced by `handle`.
    pub fn set<T: AttribValue>(
        &mut self,
        handle: &AttribHandle<T>,
        index: usize,
        value: T,
    ) -> Result<(), CoreError> {
        let key = (handle.class, handle.name.clone());
        let attr = self
            .map
            .get_mut(&key)
            .ok_or_else(|| CoreError::AttributeNotFound(handle.name.clone()))?;
        T::set_in_storage(&mut attr.storage, index, value)
    }

    /// Get a raw (untyped) reference to an attribute.
    pub fn get_raw(
        &self,
        class: AttribClass,
        name: impl AsRef<str>,
    ) -> Option<&Attribute> {
        self.map.get(&(class, name.as_ref().to_string()))
    }

    /// Get a raw (untyped) mutable reference to an attribute.
    pub fn get_raw_mut(
        &mut self,
        class: AttribClass,
        name: impl AsRef<str>,
    ) -> Option<&mut Attribute> {
        self.map.get_mut(&(class, name.as_ref().to_string()))
    }

    /// Delete an attribute. Returns true if it existed.
    pub fn delete(&mut self, class: AttribClass, name: impl AsRef<str>) -> bool {
        self.map
            .remove(&(class, name.as_ref().to_string()))
            .is_some()
    }

    /// Resize all attributes of the given class to `new_len`, filling new
    /// entries with each attribute's default value.
    pub fn resize_class(&mut self, class: AttribClass, new_len: usize) {
        for attr in self.map.values_mut() {
            if attr.class == class {
                let default = attr.default.clone();
                attr.storage.resize_with_default(new_len, &default);
            }
        }
    }

    /// List the names of all attributes for the given class.
    pub fn names(&self, class: AttribClass) -> Vec<&str> {
        self.map
            .values()
            .filter(|a| a.class == class)
            .map(|a| a.name.as_str())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_find_attribute() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([1.0, 1.0, 1.0]),
            TypeQualifier::Color,
        )
        .unwrap();

        // Prime the storage to have 3 elements
        map.resize_class(AttribClass::Point, 3);

        let handle: AttribHandle<[f32; 3]> = map.find(AttribClass::Point, "Cd").unwrap();
        let val = map.get(&handle, 0).unwrap();
        assert_eq!(val, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn set_attribute_value() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "pscale",
            AttribDefault::Float(1.0),
            TypeQualifier::None,
        )
        .unwrap();
        map.resize_class(AttribClass::Point, 4);

        let handle: AttribHandle<f32> = map.find(AttribClass::Point, "pscale").unwrap();
        map.set(&handle, 2, 3.14).unwrap();
        let val = map.get(&handle, 2).unwrap();
        assert!((val - 3.14).abs() < 1e-6);
    }

    #[test]
    fn type_mismatch_error() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "my_int",
            AttribDefault::Int(0),
            TypeQualifier::None,
        )
        .unwrap();

        // Try to find it as Float — should fail
        let result: Result<AttribHandle<f32>, _> = map.find(AttribClass::Point, "my_int");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::AttributeTypeMismatch(_)));
    }

    #[test]
    fn attribute_not_found() {
        let map = AttributeMap::new();
        let result: Result<AttribHandle<f32>, _> = map.find(AttribClass::Point, "nonexistent");
        assert!(matches!(result.unwrap_err(), CoreError::AttributeNotFound(_)));
    }

    #[test]
    fn delete_attribute() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Primitive,
            "shop_materialpath",
            AttribDefault::String(std::string::String::new()),
            TypeQualifier::None,
        )
        .unwrap();

        assert!(map.delete(AttribClass::Primitive, "shop_materialpath"));
        // Second delete returns false
        assert!(!map.delete(AttribClass::Primitive, "shop_materialpath"));
        // Should not be findable
        let result: Result<AttribHandle<std::string::String>, _> =
            map.find(AttribClass::Primitive, "shop_materialpath");
        assert!(result.is_err());
    }

    #[test]
    fn resize_class() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "pscale",
            AttribDefault::Float(2.0),
            TypeQualifier::None,
        )
        .unwrap();

        // Initial resize to 2
        map.resize_class(AttribClass::Point, 2);
        {
            let handle: AttribHandle<f32> = map.find(AttribClass::Point, "pscale").unwrap();
            assert_eq!(map.get(&handle, 0).unwrap(), 2.0);
            assert_eq!(map.get(&handle, 1).unwrap(), 2.0);
        }

        // Expand to 5
        map.resize_class(AttribClass::Point, 5);
        {
            let handle: AttribHandle<f32> = map.find(AttribClass::Point, "pscale").unwrap();
            for i in 0..5 {
                assert_eq!(map.get(&handle, i).unwrap(), 2.0, "index {i}");
            }
        }
    }

    #[test]
    fn names() {
        let mut map = AttributeMap::new();
        map.create(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([0.0; 3]),
            TypeQualifier::Color,
        )
        .unwrap();
        map.create(
            AttribClass::Point,
            "N",
            AttribDefault::Vector3([0.0; 3]),
            TypeQualifier::Normal,
        )
        .unwrap();
        map.create(
            AttribClass::Primitive,
            "shop_materialpath",
            AttribDefault::String(std::string::String::new()),
            TypeQualifier::None,
        )
        .unwrap();

        let mut point_names = map.names(AttribClass::Point);
        point_names.sort();
        assert_eq!(point_names, vec!["Cd", "N"]);

        let prim_names = map.names(AttribClass::Primitive);
        assert_eq!(prim_names, vec!["shop_materialpath"]);
    }
}
