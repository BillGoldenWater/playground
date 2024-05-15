use std::fmt::{Debug, Display};

pub trait FunctionalUtils {
    #[inline]
    fn then<R>(self, f: impl FnOnce(Self) -> R) -> R
    where
        Self: Sized,
    {
        f(self)
    }

    #[inline]
    fn then_ref<R>(self, f: impl FnOnce(&Self) -> R) -> R
    where
        Self: Sized,
    {
        f(&self)
    }

    #[inline]
    fn then_mut<R>(mut self, f: impl FnOnce(&mut Self) -> R) -> R
    where
        Self: Sized,
    {
        f(&mut self)
    }

    #[inline]
    fn then_as_ref<T, R>(self, f: impl FnOnce(&T) -> R) -> R
    where
        Self: Sized + AsRef<T>,
        T: ?Sized,
    {
        f(self.as_ref())
    }

    #[inline]
    fn then_as_mut<T, R>(mut self, f: impl FnOnce(&mut T) -> R) -> R
    where
        Self: Sized + AsMut<T>,
        T: ?Sized,
    {
        f(self.as_mut())
    }

    #[inline]
    fn with(mut self, f: impl FnOnce(&mut Self)) -> Self
    where
        Self: Sized,
    {
        f(&mut self);
        self
    }

    #[inline]
    fn try_with<E>(mut self, f: impl FnOnce(&mut Self) -> Result<(), E>) -> Result<Self, E>
    where
        Self: Sized,
    {
        f(&mut self)?;
        self.into_ok()
    }

    #[inline]
    fn some(self) -> Option<Self>
    where
        Self: Sized,
    {
        Some(self)
    }

    #[inline]
    fn into_ok<E>(self) -> Result<Self, E>
    where
        Self: Sized,
    {
        Ok(self)
    }

    #[inline]
    fn into_err<T>(self) -> Result<T, Self>
    where
        Self: Sized,
    {
        Err(self)
    }

    #[inline]
    fn unit_result<T, E>(self) -> Result<(), E>
    where
        Self: ResultExt<T, E> + Sized,
    {
        self.map_unit()
    }

    #[inline]
    fn err_into<T, E, R>(self) -> Result<T, R>
    where
        Self: ResultExt<T, E> + Sized,
        E: Into<R>,
    {
        self.map_err_into()
    }

    #[inline]
    fn println(self) -> Self
    where
        Self: Sized + Display,
    {
        println!("{self}");
        self
    }

    #[inline]
    fn println_dbg(self) -> Self
    where
        Self: Sized + Debug,
    {
        println!("{self:#?}");
        self
    }

    #[inline]
    fn println_ref(&self)
    where
        Self: Display,
    {
        println!("{self}");
    }

    #[inline]
    fn println_ref_dbg(&self)
    where
        Self: Debug,
    {
        println!("{self:#?}");
    }
}

impl<T> FunctionalUtils for T {}

pub trait ResultExt<T, E> {
    fn map_unit(self) -> Result<(), E>;

    fn map_err_into<R>(self) -> Result<T, R>
    where
        E: Into<R>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    #[inline]
    fn map_unit(self) -> Result<(), E> {
        self.map(|_| ())
    }

    #[inline]
    fn map_err_into<R>(self) -> Result<T, R>
    where
        E: Into<R>,
    {
        self.map_err(Into::into)
    }
}
