/// `impl_has_type_tag!(Type, "name")` implements `HasTypeTag` for `Type` using the name `"name"`.
///
/// This assigns a stable name to the type for serialization purposes. To ensure the name is globally
/// unique, it should be a fully qualified path to the type (e.g. `"std::string::String"`). To
/// handle version skew between producers and consumers of serialized data, the name should not
/// change, even if the original type moves or changes names.
#[macro_export]
macro_rules! impl_any_serde {
    ($ty:ty, $($name:tt)*) => {
        impl $crate::tag::HasTypeTag for $ty {
            fn type_tag() -> &'static $crate::tag::TypeTag {
                #[allow(non_upper_case_globals, non_snake_case)]
                mod  internal  {
                    use lazy_static::lazy_static;
                    lazy_static! {
                        pub static ref TYPE_TAG: $crate::tag::TypeTag = $crate::tag::TypeTag::new($($name)*);
                    }
                }
                &*internal::TYPE_TAG
            }
        }
        impl $crate::AnySerde for $ty {
            fn clone_box(&self) -> $crate::BoxAnySerde{
                Box::new(self.clone())
            }
        }
    }
}

/// `impl_any_json!(T)` registers `T` for use in [`AnySerde`](crate::AnySerde) with the JSON format.
#[macro_export]
macro_rules! impl_any_json {
    ($ty:ty) => {
        impl $crate::JsonNopTrait for $ty {
            fn nop(){
                static SINGLETON: $crate::util::AnySingleton<$ty> = $crate::util::AnySingleton::new();
                use $crate::reexport::inventory;
                inventory::submit!(&SINGLETON as &'static dyn $crate::json::AnyJson);
            }
        }
    }
}

/// `impl_any_json!(T)` registers `T` for use in [`AnySerde`](crate::AnySerde) with the binary format.
#[macro_export]
macro_rules! impl_any_binary {
    ($ty:ty) => {
        impl $crate::BinaryNopTrait for $ty{
            fn nop(){
                static SINGLETON: $crate::util::AnySingleton<$ty> = $crate::util::AnySingleton::new();
                use $crate::reexport::inventory;
                inventory::submit!(&SINGLETON as &'static dyn $crate::binary::any::AnyBinary);
            }
        }
    }
}