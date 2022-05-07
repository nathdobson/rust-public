/// `impl_has_type_tag!(Type, "name")` implements `HasTypeTag` for `Type` using the name `"name"`.
///
/// This assigns a stable name to the type for serialization purposes. To ensure the name is globally
/// unique, it should be a fully qualified path to the type (e.g. `"std::string::String"`). To
/// handle version skew between producers and consumers of serialized data, the name should not
/// change, even if the original type moves or changes names.
#[macro_export]
macro_rules! impl_any_serde {
    ($ty:ty, $name:tt, $($registry:path),*) => {
        impl $crate::tag::HasTypeTag for $ty {
            fn type_tag() -> &'static $crate::tag::TypeTag {
                #[allow(non_upper_case_globals, non_snake_case)]
                mod  internal  {
                    pub static TYPE_TAG: ::std::lazy::SyncLazy<$crate::tag::TypeTag>
                        = ::std::lazy::SyncLazy::new(|| $crate::tag::TypeTag::new($name));
                }
                &*internal::TYPE_TAG
            }
        }
        impl $crate::AnySerde for $ty {
            fn clone_box(&self) -> $crate::BoxAnySerde{
                Box::new(self.clone())
            }
            fn inner_type_name(&self) -> &'static str{
                ::std::any::type_name::<Self>()
            }
        }
        $(
            const _ : () = {
                #[$crate::reexport::catalog::register($registry,crate=$crate::reexport::catalog)]
                fn a() -> &'static ::std::marker::PhantomData<$ty> { &::std::marker::PhantomData }
            };
        )*
    }
}
