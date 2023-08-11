///
/// whether to use rayon or not when using iter
///

// iter
// choose which iter implementation to use
#[cfg(feature = "threading")]
#[macro_export]
macro_rules! iter {
    ($data:expr) => {{
        use rayon::prelude::*;
        $data.par_iter()
    }};
}
#[cfg(not(feature = "threading"))]
#[macro_export]
macro_rules! iter {
    ($data:expr) => {
        $data.iter()
    };
}

// sort
#[cfg(feature = "threading")]
#[macro_export]
macro_rules! sort {
    ($data:expr) => {{
        use rayon::prelude::*;
        $data.par_sort()
    }};
}
#[cfg(not(feature = "threading"))]
#[macro_export]
macro_rules! sort {
    ($data:expr) => {{
        $data.sort()
    }};
}

// sort_reverse
#[cfg(feature = "threading")]
#[macro_export]
macro_rules! sort_reverse {
    ($data:expr) => {{
        use rayon::prelude::*;
        $data.par_sort_by(|a, b| b.cmp(a))
    }};
}
#[cfg(not(feature = "threading"))]
#[macro_export]
macro_rules! sort_reverse {
    ($data:expr) => {{
        $data.sort_by(|a, b| b.cmp(a))
    }};
}
