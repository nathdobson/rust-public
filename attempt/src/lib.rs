#![feature(try_trait)]

use std::ops::Try;

pub fn try_into_result<T: Try>(element: T) -> Result<T::Ok, T::Error> {
    element.into_result()
}

#[macro_export]
macro_rules! attempt {
    ( $inner:expr ; catch ($($infallible:tt)*) => $handle:expr ) => {{
        #[allow(unused_mut)]
        let mut result;
        #[allow(unused_labels)]
        'catch: loop {
            #[allow(unused_macros)]
            macro_rules! catch {
                ($possible:expr) => {
                    match $crate::try_into_result($possible){
                        Ok(x) => x,
                        Err(error) => {
                            result = Err(std::convert::From::from(error));
                            break 'catch;
                        }
                    }
                }
            }
            result = Ok($inner);
            break;
        }
        match result {
            Ok(value) => value,
            Err(error) => {
                let $($infallible)* = error;
                $handle
            } ,
        }
    }}
}

#[test]
fn sum_option_test() {
    use std::option::NoneError;
    use std::{ops, result};
    fn sum_option_with_ascription<T: ops::Add<Output=T>>(x: Option<T>, y: Option<T>) -> Option<T> {
        return attempt!({
            Some(catch!(x) + catch!(y))
        }; catch (_: NoneError) => {
            None
        });
    }

    fn sum_option_with_pattern<T: ops::Add<Output=T>>(x: Option<T>, y: Option<T>) -> Option<T> {
        return attempt!({
            Some(catch!(x) + catch!(y))
        }; catch (NoneError) => {
            None
        });
    }

    fn sum_option_with_throw<T: ops::Add<Output=T>>(x: Option<T>, y: Option<T>) -> result::Result<T, NoneError> {
        return attempt!({
            Ok(catch!(x) + catch!(y))
        }; catch (e) => {
            Err(e)
        });
    }
    assert_eq!(sum_option_with_ascription::<usize>(None, None), None);
    assert_eq!(sum_option_with_ascription(None, Some(1)), None);
    assert_eq!(sum_option_with_ascription(Some(1), None), None);
    assert_eq!(sum_option_with_ascription(Some(1), Some(1)), Some(2));
    assert_eq!(sum_option_with_pattern::<usize>(None, None), None);
    assert_eq!(sum_option_with_pattern(None, Some(1)), None);
    assert_eq!(sum_option_with_pattern(Some(1), None), None);
    assert_eq!(sum_option_with_pattern(Some(1), Some(1)), Some(2));
    assert_eq!(sum_option_with_throw::<usize>(None, None), Err(NoneError));
    assert_eq!(sum_option_with_throw(None, Some(1)), Err(NoneError));
    assert_eq!(sum_option_with_throw(Some(1), None), Err(NoneError));
    assert_eq!(sum_option_with_throw(Some(1), Some(1)), Ok(2));
}