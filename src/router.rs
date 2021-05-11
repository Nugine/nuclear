use crate::error::NotFound;
use crate::http::Method;
use crate::internal_prelude::*;

use std::ops::{Range, RangeFrom};
use std::str::FromStr;

use smallvec::SmallVec;

#[derive(Default)]
pub struct SimpleRouter {
    router: Router,
    effects: Vec<Box<dyn Handler>>,
    default: Option<Box<dyn Handler>>,
}

pub struct CaptureOwned {
    path: Box<str>,
    captures: Captures,
}

impl CaptureOwned {
    fn get_param(&self, name: &str) -> Option<&str> {
        self.captures.get_param(self.path.as_ref(), name)
    }
}

pub trait SimpleRouterExt {
    fn capture(&self) -> Option<&CaptureOwned>;

    fn param(&self, name: &str) -> Option<&str> {
        self.capture()?.get_param(name)
    }

    #[track_caller]
    fn expect_param(&self, name: &str) -> &str {
        match self.capture() {
            Some(c) => match c.get_param(name) {
                Some(s) => s,
                None => panic!("param {:?} not found", name),
            },
            None => panic!("capture not found"),
        }
    }

    fn parse_param<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        self.param(name).map(FromStr::from_str)
    }
}

impl SimpleRouterExt for Request {
    fn capture(&self) -> Option<&CaptureOwned> {
        self.extensions().get::<CaptureOwned>()
    }
}

impl SimpleRouter {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            default: None,
            router: Router::new(),
        }
    }

    pub fn set_default(&mut self, h: Box<dyn Handler>) {
        self.default = Some(h);
    }

    pub fn at(&mut self, pattern: &'static str) -> RouteSetter<'_> {
        RouteSetter {
            router: self,
            pattern,
        }
    }

    pub fn add_route(&mut self, methods: &[Method], pattern: &'static str, h: Box<dyn Handler>) {
        let idx = self.effects.len();
        self.router.add_route(methods, pattern, idx, true);
        self.effects.push(h);
    }

    pub fn find(&self, method: &Method, path: &str) -> Option<(&dyn Handler, CaptureOwned)> {
        let mut captures = Captures::empty();
        let idx = self.router.find(&mut captures, method, path)?;
        let capture_owned = CaptureOwned {
            path: path.into(),
            captures,
        };
        let f = &*self.effects[idx];
        Some((f, capture_owned))
    }
}

impl Handler for SimpleRouter {
    fn handle<'t, 'a>(&'t self, mut req: Request) -> BoxFuture<'a, Result<Response>>
    where
        't: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let hreq = &mut *req;
            let method = hreq.method();
            let path = hreq.uri().path();
            match self.find(method, path) {
                Some((h, capture)) => {
                    let _ = hreq.extensions_mut().insert(capture);
                    h.handle(req).await
                }
                None => match self.default.as_ref() {
                    Some(h) => h.handle(req).await,
                    None => Ok(NotFound.into()),
                },
            }
        })
    }
}

pub struct RouteSetter<'r> {
    router: &'r mut SimpleRouter,
    pattern: &'static str,
}

macro_rules! define_method {
    {$name:ident, $method:expr} => {
        pub fn $name(&mut self, h: Box<dyn Handler>) -> &mut Self {
            self.router.add_route(&[$method], self.pattern, h);
            self
        }
    };
}

impl RouteSetter<'_> {
    define_method! {get, Method::GET}
    define_method! {post, Method::POST}
    define_method! {put, Method::PUT}
    define_method! {delete, Method::DELETE}
    define_method! {head, Method::HEAD}
    define_method! {options, Method::OPTIONS}
    define_method! {connect, Method::CONNECT}
    define_method! {patch, Method::PATCH}
    define_method! {trace, Method::TRACE}
}

#[derive(Default)]
struct Router {
    routes: Vec<Route>,
}

struct Route {
    segments: Box<[Segment]>,
    catch_tail: bool,
    data_index: usize,
    method_mask: u16,
}

enum Segment {
    Static(&'static str),
    Capture(&'static str),
}

#[derive(Debug)]
struct Captures {
    params: Option<Vec<(&'static str, Range<usize>)>>,
    tail: Option<RangeFrom<usize>>,
}

const METHODS: [Method; 9] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::HEAD,
    Method::OPTIONS,
    Method::CONNECT,
    Method::PATCH,
    Method::TRACE,
];

fn to_index(method: &Method) -> u8 {
    for (i, m) in METHODS.iter().enumerate() {
        if m == method {
            return i as u8;
        }
    }

    panic!("unsupported method: {:?}", method);
}

impl Route {
    fn try_match<'p>(
        &self,
        captures: &mut Captures,
        path: &'p str,
        parts: &[&'p str],
    ) -> Option<usize> {
        let segment_num: usize = self.segments.len() + self.catch_tail as usize;
        if segment_num > parts.len() {
            return None;
        }

        if self.segments.len() < parts.len() && !self.catch_tail {
            return None;
        }

        let params = captures.params.get_or_insert_with(Vec::new);
        let origin_len = params.len();

        let iter = self.segments.iter().zip(parts.iter());
        let mut tail_start: usize = 0;
        for (segment, &part) in iter {
            tail_start += part.len() + 1;
            match *segment {
                Segment::Static(s) => {
                    if s != part {
                        params.truncate(origin_len);
                        return None;
                    }
                }
                Segment::Capture(name) => {
                    let range = calc_range(path, part);
                    params.push((name, range));
                }
            }
        }

        captures.tail = if self.catch_tail {
            Some(tail_start..)
        } else {
            None
        };

        Some(self.data_index)
    }
}

fn calc_range(base: &str, part: &str) -> Range<usize> {
    let start = (part.as_ptr() as usize) - (base.as_ptr() as usize);
    let end = start + part.len();
    start..end
}

impl Router {
    fn new() -> Self {
        Self { routes: Vec::new() }
    }

    fn find(&self, captures: &mut Captures, method: &Method, path: &str) -> Option<usize> {
        assert!(path.starts_with('/'));
        let parts: SmallVec<[&str; 4]> = path.split('/').skip(1).collect();

        let mask: u16 = 1_u16 << to_index(method);

        for route in self.routes.iter() {
            if route.method_mask & mask == 0 {
                continue;
            }
            if let Some(index) = route.try_match(captures, path, &parts) {
                return Some(index);
            }
        }

        None
    }

    fn add_route(
        &mut self,
        methods: &[Method],
        pattern: &'static str,
        data_index: usize,
        allow_tail: bool,
    ) {
        assert!(pattern.starts_with('/'));
        let mut segments: Vec<&str> = pattern.split('/').skip(1).collect();
        let catch_tail = if *segments.last().unwrap() == "**" {
            if !allow_tail {
                panic!("pattern {:?} can not contain tail wildcard", pattern);
            }
            segments.pop();
            true
        } else {
            false
        };

        let segments: Box<[Segment]> = segments
            .into_iter()
            .map(|s| match s.as_bytes() {
                [b':', ..] => Segment::Capture(&s[1..]),
                _ => Segment::Static(s),
            })
            .collect();

        let method_mask = methods
            .iter()
            .fold(0_u16, |acc, m| acc | (1_u16 << to_index(m)));

        let route: Route = Route {
            segments,
            catch_tail,
            data_index,
            method_mask,
        };

        self.routes.push(route);
    }
}

impl Captures {
    fn empty() -> Self {
        Self {
            params: None,
            tail: None,
        }
    }

    fn get_param<'p>(&self, path: &'p str, name: &str) -> Option<&'p str> {
        let params = self.params.as_deref()?;
        for &(n, ref range) in params.iter() {
            if n == name {
                return Some(&path[range.clone()]);
            }
        }
        None
    }
}

#[test]
fn simple_router() {
    let mut router = Router::new();

    const GET: Method = Method::GET;
    const POST: Method = Method::POST;

    router.add_route(&[POST], "/posts", 1, true);
    router.add_route(&[GET, POST], "/posts/:pid", 2, true);
    router.add_route(&[GET], "/static/**", 3, true);

    let mut captures = Captures::empty();

    assert_eq!(router.find(&mut captures, &GET, "/posts/asd"), Some(2));
    assert_eq!(router.find(&mut captures, &POST, "/posts/asd"), Some(2));

    assert_eq!(router.find(&mut captures, &GET, "/posts/"), Some(2));
    assert_eq!(router.find(&mut captures, &POST, "/posts/"), Some(2));

    assert_eq!(router.find(&mut captures, &GET, "/posts"), None);
    assert_eq!(router.find(&mut captures, &POST, "/posts"), Some(1));

    assert_eq!(router.find(&mut captures, &GET, "/static"), None);
    assert_eq!(router.find(&mut captures, &GET, "/static/"), Some(3));
    assert_eq!(router.find(&mut captures, &GET, "/static/asd"), Some(3));

    dbg!(&captures);
}
