pub mod any_string;
pub mod custom;

use registry::registry;
registry!{
    require any_string;
    require typetag_static;
}