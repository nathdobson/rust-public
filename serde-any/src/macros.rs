#[macro_export]
macro_rules! impl_any_serde {
    ($ty:ident, $name:literal) => {
        paste::paste! {
            #[allow(non_upper_case_globals, non_snake_case)]
            mod [< __TYPE_TAG_MODULE__ $ty >] {
                use lazy_static::lazy_static;
                lazy_static! {
                    pub static ref TYPE_TAG: $crate::tag::TypeTag = $crate::tag::TypeTag::new($name);
                }
            }
            inventory::submit! {
                $crate::de::AnyDeserializeEntry::new::<$ty>()
            }
            impl $crate::tag::HasTypeTag for $ty {
                fn type_tag() -> &'static $crate::tag::TypeTag {
                    &*[< __TYPE_TAG_MODULE__ $ty >]::TYPE_TAG
                }
            }
        }
    }
}