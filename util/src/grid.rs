use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::rect::Rect;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Grid<T> {
    size: (isize, isize),
    vec: Vec<T>,
}

impl<T> Default for Grid<T> {
    fn default() -> Self {
        Grid {
            size: (0, 0),
            vec: vec![],
        }
    }
}

pub struct GridSliceRaw<T> {
    grid: NonNull<Grid<T>>,
    bounds: Rect,
}

pub struct GridSlice<'a, T>(GridSliceRaw<T>, PhantomData<&'a T>);

pub struct GridSliceMut<'a, T>(GridSliceRaw<T>, PhantomData<&'a mut T>);

//pub struct GridSliceDrain<'a, T>(GridSliceRaw<T>, PhantomData<&'a T>);

pub struct Rows<S: GridSliceIndex>(Option<S>);

pub struct Cols<S: GridSliceIndex>(Option<S>);


impl<T> Grid<T> {
    pub fn from_iterator(size: (isize, isize), iter: impl Iterator<Item=T>) -> Self {
        let vec: Vec<T> = iter.collect();
        assert!(size.0 * size.1 == vec.len() as isize);
        Grid { size, vec }
    }
    pub fn new(size: (isize, isize), mut f: impl FnMut(isize, isize) -> T) -> Self {
        let mut vec = Vec::with_capacity((size.0 * size.1) as usize);
        for y in 0..size.1 {
            for x in 0..size.0 {
                vec.push(f(x, y));
            }
        }
        Grid { size, vec }
    }
    fn index_of(&self, p: (isize, isize)) -> Option<usize> {
        if p.0 >= 0 && p.0 < self.size.0 && p.1 >= 0 && p.1 < self.size.1 {
            Some((p.0 + p.1 * self.size.0) as usize)
        } else {
            None
        }
    }
    fn as_raw(&self) -> GridSliceRaw<T> {
        GridSliceRaw {
            grid: NonNull::from(self),
            bounds: self.bounds(),
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::from_ranges(0..self.size.0, 0..self.size.1)
    }
    pub fn as_ref(&self) -> GridSlice<T> {
        GridSlice(self.as_raw(), PhantomData)
    }
    pub fn as_mut(&mut self) -> GridSliceMut<T> {
        GridSliceMut(self.as_raw(), PhantomData)
    }
    //    pub fn drain(&mut self) -> GridSliceDrain<T> {
//        unsafe {
//            let bounds = self.bounds();
//            let (ptr, _, cap) = mem::replace(&mut self.vec, vec![]).into_raw_parts();
//            self.vec = Vec::from_raw_parts(ptr, 0, cap);
//            self.size.1 = 0;
//            GridSliceDrain(GridSliceRaw {
//                grid: NonNull::from(self),
//                bounds,
//            }, PhantomData)
//        }
//    }
    pub fn get(&self, p: (isize, isize)) -> Option<&T> {
        self.vec.get(self.index_of(p)?)
    }
    pub fn get_mut(&mut self, p: (isize, isize)) -> Option<&mut T> {
        let index = self.index_of(p)?;
        self.vec.get_mut(index)
    }
    pub fn size(&self) -> (isize, isize) {
        self.size
    }
    pub fn into_iter(self) -> impl Iterator<Item=((isize, isize), T)> {
        self.bounds().points_by_row().zip(self.vec.into_iter())
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item=((isize, isize), &mut T)> {
        self.bounds().points_by_row().zip(self.vec.iter_mut())
    }
    pub fn iter(&self) -> impl Iterator<Item=((isize, isize), &T)> {
        self.bounds().points_by_row().zip(self.vec.iter())
    }
    pub fn points(&self) -> impl Iterator<Item=(isize, isize)> {
        self.bounds().points_by_row()
    }
    pub fn values(&self) -> impl Iterator<Item=&T> {
        self.vec.iter()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut T> {
        self.vec.iter_mut()
    }
}

impl<T> Copy for GridSliceRaw<T> {}

impl<T> Clone for GridSliceRaw<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for GridSlice<'a, T> {}

impl<'a, T> Clone for GridSlice<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait GridSliceIndex: Sized {
    type Source;
    type Target;
    fn as_raw(&self) -> &GridSliceRaw<Self::Source>;
    unsafe fn from_raw(raw: GridSliceRaw<Self::Source>) -> Self;
    fn split(self, rects: &mut [(Rect, Option<Self>)]) {
        unsafe {
            let this = self.as_raw();
            for (i, x) in rects.iter().enumerate() {
                for y in rects[i + 1..].iter() {
                    assert!(!x.0.intersects(&y.0))
                }
            }
            for (r, o) in rects.iter_mut() {
                *o = Some(Self::from_raw(GridSliceRaw {
                    grid: this.grid,
                    bounds: this.bounds.sub_rectangle(r),
                }));
            }
        }
    }
    fn with_bounds(self, rect: Rect) -> Self {
        unsafe {
            let this = self.as_raw();
            Self::from_raw(GridSliceRaw {
                grid: this.grid,
                bounds: this.bounds.sub_rectangle(&rect),
            })
        }
    }
    fn with_bounds_truncated(self, rect: Rect) -> Self {
        unsafe {
            let this = self.as_raw();
            Self::from_raw(GridSliceRaw {
                grid: this.grid,
                bounds: this.bounds.sub_rectangle_truncated(&rect),
            })
        }
    }
    fn size(&self) -> (isize, isize) {
        self.as_raw().bounds.size()
    }
    fn cell(self) -> Self::Target;
}

pub fn rows<T: GridSliceIndex>(x: T) -> Rows<T> {
    Rows(Some(x))
}

pub fn cols<T: GridSliceIndex>(x: T) -> Cols<T> {
    Cols(Some(x))
}

pub fn cells_by_row<T: GridSliceIndex>(x: T) -> impl Iterator<Item=T::Target> {
    rows(x).flat_map(|y| cols(y).map(|z| z.cell()))
}

pub fn cells_by_col<T: GridSliceIndex>(x: T) -> impl Iterator<Item=T::Target> {
    cols(x).flat_map(|y| rows(y).map(|z| z.cell()))
}

impl<'a, T> GridSliceIndex for GridSlice<'a, T> {
    type Source = T;
    type Target = &'a T;
    fn as_raw(&self) -> &GridSliceRaw<T> { &self.0 }
    unsafe fn from_raw(x: GridSliceRaw<T>) -> Self { GridSlice(x, PhantomData) }
    fn cell(self) -> Self::Target {
        unsafe { &*self.0.get_raw((0, 0)).unwrap().as_ptr() }
    }
}

impl<'a, T> GridSliceIndex for GridSliceMut<'a, T> {
    type Source = T;
    type Target = &'a mut T;
    fn as_raw(&self) -> &GridSliceRaw<T> { &self.0 }
    unsafe fn from_raw(x: GridSliceRaw<T>) -> Self { GridSliceMut(x, PhantomData) }
    fn cell(self) -> Self::Target {
        unsafe { &mut *self.0.get_raw((0, 0)).unwrap().as_ptr() }
    }
}

//impl<'a, T> GridSliceIndex for GridSliceDrain<'a, T> {
//    type Source = T;
//    type Target = T;
//    fn as_raw(&self) -> &GridSliceRaw<T> { &self.0 }
//    unsafe fn from_raw(x: GridSliceRaw<T>) -> Self { GridSliceDrain(x, PhantomData) }
//    fn cell(self) -> Self::Target {
//        unsafe { ptr::read(self.0.get_raw((0, 0)).unwrap().as_ptr()) }
//    }
//}

impl<'a, T> GridSliceMut<'a, T> {
    pub fn as_mut<'b>(&'b mut self) -> GridSliceMut<'b, T> {
        GridSliceMut(self.0, PhantomData)
    }
    pub fn into_ref(self) -> GridSlice<'a, T> {
        GridSlice(self.0, PhantomData)
    }
    pub fn as_ref<'b>(&'b self) -> GridSlice<'b, T> {
        GridSlice(self.0, PhantomData)
    }
    pub fn get_mut<'b>(&'b mut self, index: (isize, isize)) -> Option<&'b mut T> {
        self.0.get_raw(index).map(|ptr| unsafe { &mut *ptr.as_ptr() })
    }
}

//impl<'a, T> GridSliceDrain<'a, T> {
//    pub fn as_ref<'b>(&'b self) -> GridSlice<'b, T> {
//        GridSlice(self.0, PhantomData)
//    }
//    pub fn into_ref(self) -> GridSlice<'a, T> {
//        GridSlice(self.0, PhantomData)
//    }
//}


impl<S: GridSliceIndex> Iterator for Rows<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.take()?;
        let (w, h) = inner.size();
        if h == 0 {
            return None;
        }
        let mut results = [(Rect::from_ranges(0..w, 0..1), None), (Rect::from_ranges(0..w, 1..h), None)];
        inner.split(&mut results);
        self.0 = Some(results[1].1.take().unwrap());
        results[0].1.take()
    }
}

impl<S: GridSliceIndex> Iterator for Cols<S> {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.take()?;
        let (w, h) = inner.size();
        if w == 0 {
            return None;
        }
        let mut results = [(Rect::from_ranges(0..1, 0..h), None), (Rect::from_ranges(1..w, 0..h), None)];
        inner.split(&mut results);
        self.0 = Some(results[1].1.take().unwrap());
        results[0].1.take()
    }
}

impl<T> GridSliceRaw<T> {
    fn get_raw(&self, index: (isize, isize)) -> Option<NonNull<T>> {
        unsafe {
            let grid = self.grid.as_ref();
            let index = grid.index_of(self.bounds.translate(index)?)?;
            NonNull::new(grid.vec.as_ptr().add(index) as *mut T)
        }
    }
}

impl<'a, T> Index<(isize, isize)> for GridSlice<'a, T> {
    type Output = T;

    fn index(&self, index: (isize, isize)) -> &Self::Output {
        unsafe { &*self.0.get_raw(index).unwrap().as_ptr() }
    }
}

impl<'a, T> Index<(isize, isize)> for GridSliceMut<'a, T> {
    type Output = T;

    fn index(&self, index: (isize, isize)) -> &Self::Output {
        unsafe { &*self.0.get_raw(index).unwrap().as_ptr() }
    }
}

impl<'a, T> IndexMut<(isize, isize)> for GridSliceMut<'a, T> {
    fn index_mut(&mut self, index: (isize, isize)) -> &mut Self::Output {
        unsafe {
            &mut *self.0.get_raw(index).unwrap().as_ptr()
        }
    }
}

impl<T> Index<(isize, isize)> for Grid<T> {
    type Output = T;

    fn index(&self, index: (isize, isize)) -> &Self::Output {
        &self.vec[self.index_of(index).unwrap()]
    }
}

impl<T> IndexMut<(isize, isize)> for Grid<T> {
    fn index_mut(&mut self, index: (isize, isize)) -> &mut Self::Output {
        let index = self.index_of(index);
        &mut self.vec[index.unwrap()]
    }
}

#[test]
fn test_grid_index() {
    let foo = Grid::new((2, 3), |x, y| Box::new(x + y * 2));
    assert_eq!(*foo[(0, 0)], 0);
    assert_eq!(*foo[(1, 0)], 1);
    assert_eq!(*foo[(0, 1)], 2);
    assert_eq!(*foo[(1, 1)], 3);
    assert_eq!(*foo[(0, 2)], 4);
    assert_eq!(*foo[(1, 2)], 5);
}

#[test]
fn test_grid_rows() {
    let foo = Grid::new((2, 3), |x, y| Box::new(x + y * 2));
    println!("{:?}", rows(foo.as_ref()).map(|x| x.0.bounds).collect::<Vec<_>>());
}

#[test]
fn test_grid_cells_by_row() {
    let mut foo = Grid::new((2, 3), |x, y| Box::new(x + y * 2));
    assert_eq!((0..6).map(Box::new).collect::<Vec<_>>(), cells_by_row(foo.as_ref()).cloned().collect::<Vec<_>>());
    assert_eq!((0..6).map(Box::new).collect::<Vec<_>>(), cells_by_row(foo.as_mut()).map(|x| x.clone()).collect::<Vec<_>>());
    //assert_eq!((0..6).map(Box::new).collect::<Vec<_>>(), cells_by_row(foo.drain()).collect::<Vec<_>>());
    assert_eq!(Vec::<&Box<_>>::new(), cells_by_row(foo.as_ref()).collect::<Vec<_>>());
    assert_eq!(Vec::<&Box<_>>::new(), cells_by_row(foo.as_mut()).collect::<Vec<_>>());
}
