#[macro_export]
macro_rules! cfg_kinds {
    (if flat in [flat $(, $kind:ident)*] {$($a:item)*} $(else {$($b:item)*})?) => { $($a)* };
    (if serde in [serde $(, $kind:ident)*] {$($a:item)*} $(else {$($b:item)*})?) => { $($a)* };
    (if $x:ident in [] {$($a:tt)*} $(else {$($b:item)*})?) => { $($($b)*)? };
    (if $x:ident in [$other:ident $(, $kind:ident)*] {$($a:item)*} $(else {$($b:item)*})?) => {
        cfg_kinds!(if $x in [$($kind),*] { $($a)* } $(else { $($b)* })?)
    };
}

macro_rules! type_tag {
    (
        type $ty:tt $(<$lt:tt>)*,
        name $name:literal,
        kinds $kinds:tt
    ) => {{
        use $crate::reexport::lazy_static::lazy_static;
        use $crate::tag::FlatTypeTag;
        use $crate::tag::TypeTag;
        use $crate::tag::HasFlatTypeTag;
        use $crate::tag::HasTypeTag;
        use ::std::marker::PhantomData;

        $crate::cfg_kinds!{
            if flat in $kinds {
                static FLAT_TAG: FlatTypeTag = FlatTypeTag::new::<$ty>();
                unsafe impl $(<$lt>)? HasFlatTypeTag for $ty $(<$lt>)? {
                    fn flat_type_tag() -> &'static FlatTypeTag{ &FLAT_TAG }
                }
                static FLAT_TAG_OPTION: Option<FlatTypeTag> = Some(FLAT_TAG);
            } else {
                static FLAT_TAG_OPTION: Option<FlatTypeTag> = None;
            }
        };
        lazy_static! {
            static ref TYPE_TAG: TypeTag = TypeTag::new::<$ty>($name, FLAT_TAG_OPTION);
        }
        impl $(<$lt>)* HasTypeTag for $ty $(<$lt>)*{
            fn type_tag() -> &'static TypeTag { &TYPE_TAG }
        }
        PhantomData::<$ty>
    }}
}