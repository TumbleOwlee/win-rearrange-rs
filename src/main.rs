use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_ulong};
use structopt::StructOpt;
use x11::xlib::{
    Display as XDisplay, Window as XWindow, XCloseDisplay, XDefaultScreen, XGetWMName, XTextProperty,
    XGetWindowAttributes, XMapWindow, XMoveResizeWindow, XOpenDisplay, XQueryTree, XRaiseWindow,
    XRootWindow, XUnmapWindow, XWindowAttributes,
};

// Context holding basic references to XRoot
struct Context {
    display: std::rc::Rc<*mut XDisplay>,
    _screen: c_int,
    root: c_ulong,
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(*self.display) };
    }
}

// Window container structure holding all windows
struct WindowContainer<'a> {
    context: &'a Context,
    windows: Vec<XWindow>,
}

// Data of window
struct Window {
    name: String,
    attr: XWindowAttributes,
    window: XWindow,
    display: std::rc::Rc<*mut XDisplay>,
}

// Implement method on window
impl<'a> Window {
    pub fn name(&'a self) -> &'a String{
        &self.name
    }

    pub fn attr(&'a self) -> &'a XWindowAttributes {
        &self.attr
    }

    pub fn move_and_resize(&mut self, pos_x: i32, pos_y: i32, width: i32, height: i32) {
        // Update current state to keep up to date
        self.attr.x = pos_x;
        self.attr.y = pos_y;
        self.attr.width = width;
        self.attr.height = height;

        unsafe {
            XMoveResizeWindow(
                *self.display,
                self.window,
                pos_x,
                pos_y,
                width as u32,
                height as u32,
            );
        }
    }

    pub fn raise(&self) {
        unsafe {
            XRaiseWindow(*self.display, self.window);
        }
    }

    pub fn hide(&self) {
        unsafe {
            XUnmapWindow(*self.display, self.window);
        }
    }

    pub fn show(&self) {
        unsafe {
            XMapWindow(*self.display, self.window);
        }
    }

    pub fn resync(&mut self) -> Result<(), ()> {
        // Get window name
        let mut name = unsafe { MaybeUninit::<XTextProperty>::uninit().assume_init() };
        // Get window attributes
        let mut attr = unsafe { MaybeUninit::<XWindowAttributes>::uninit().assume_init() };
        // Get window name
        if 0 == unsafe { XGetWMName(*self.display, self.window, std::ptr::addr_of_mut!(name)) }
            || 0 == unsafe {
                XGetWindowAttributes(*self.display, self.window, std::ptr::addr_of_mut!(attr))
            } || name.format != 8
        {
            return Err(());
        }
        // Update name
        self.name = unsafe { String::from_raw_parts(name.value, name.nitems as usize, name.nitems as usize) };
        self.attr = attr;
        Ok(())
    }
}

// Iterator allowing to interate over all valid windows
struct WindowContainerIterator<'a> {
    container: WindowContainer<'a>,
    idx: usize,
}

// Create IntoIterator for all windows
impl<'a> IntoIterator for WindowContainer<'a> {
    type Item = Window;
    type IntoIter = WindowContainerIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        WindowContainerIterator { container: self, idx: 0 }
    }
}

// Custom iterator of child window data, only returning windows with valid data
impl<'a> Iterator for WindowContainerIterator<'a> {
    type Item = Window;
    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.container.windows.len() {
            // Get window id
            let window = self.container.windows[self.idx];
            self.container.windows[self.idx] = 0;
            self.idx += 1;
            // Get window name
            let mut name = unsafe { MaybeUninit::<XTextProperty>::uninit().assume_init() };
            // Get window attributes
            let mut attr = unsafe { MaybeUninit::<XWindowAttributes>::uninit().assume_init() };
            // Get window name
            if 0 == unsafe {
                XGetWMName(
                    *self.container.context.display,
                    window,
                    std::ptr::addr_of_mut!(name),
                )
            } || 0
                == unsafe {
                    XGetWindowAttributes(
                        *self.container.context.display,
                        window,
                        std::ptr::addr_of_mut!(attr),
                    )
                }
                || name.format != 8
            {
                continue;
            }
            // Create null terminated string
            let name = unsafe { String::from_raw_parts(name.value, name.nitems as usize, name.nitems as usize) };
            // Return window data
            return Some(Window {
                name,
                attr,
                window,
                display: self.container.context.display.clone(),
            });
        }
        None
    }
}

impl Context {
    pub fn new() -> Self {
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            let _screen = XDefaultScreen(display);
            let root = XRootWindow(display, _screen);
            Self {
                display: std::rc::Rc::new(display),
                _screen,
                root,
            }
        }
    }

    fn children(&self, window: XWindow) -> Result<Vec<XWindow>, ()> {
        // Initialize data for XQueryTree
        let (mut root, mut parent, mut children, mut num_children): (
            XWindow,
            XWindow,
            *mut XWindow,
            u32,
        ) = (0, 0, std::ptr::null_mut(), 0);
        // Get tree
        unsafe {
            if 0 != XQueryTree(
                *self.display,
                window,
                std::ptr::addr_of_mut!(root),
                std::ptr::addr_of_mut!(parent),
                std::ptr::addr_of_mut!(children),
                &mut num_children,
            ) {
                let mut v = Vec::from_raw_parts(
                    children,
                    num_children as usize,
                    num_children as usize,
                );
                let mut c = v.iter().map(|c| self.children(*c)).filter_map(|r| r.ok()).flatten().collect();
                v.append(&mut c);
                Ok(v)
            } else {
                Err(())
            }
        }
    }

    pub fn windows(&self) -> Result<WindowContainer, ()> {
        self.children(self.root).map(|r| WindowContainer { context: self, windows: r })
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "Opt")]
enum Opt {
    Resize {
        #[structopt(name = "REGEX")]
        regex: String,
        #[structopt(long)]
        width: i32,
        #[structopt(long)]
        height: i32,
    },
    Move {
        #[structopt(name = "REGEX")]
        regex: String,
        #[structopt(short = "x")]
        pos_x: i32,
        #[structopt(short = "y")]
        pos_y: i32,
    },
    Show {
        #[structopt(name = "REGEX")]
        regex: String,
    },
    Hide {
        #[structopt(name = "REGEX")]
        regex: String,
    },
    Raise {
        #[structopt(name = "REGEX")]
        regex: String,
    },
}

fn main() {
    // Parse commandline
    let opt = Opt::from_args();
    // Create regex
    let re = match opt {
        Opt::Resize { ref regex, .. } => regex::Regex::new(regex).unwrap(),
        Opt::Move { ref regex, .. } => regex::Regex::new(regex).unwrap(),
        Opt::Show { ref regex } => regex::Regex::new(regex).unwrap(),
        Opt::Hide { ref regex } => regex::Regex::new(regex).unwrap(),
        Opt::Raise { ref regex } => regex::Regex::new(regex).unwrap(),
    };
    // Get context and window container
    let context = Context::new();
    let container = context.windows().unwrap();
    // Iterate over all windows and apply command
    for mut w in container.into_iter() {
        if re.captures(w.name()).is_some() {
            match opt {
                Opt::Resize {
                    regex: _,
                    width,
                    height,
                } => w.move_and_resize(w.attr().x, w.attr().y, width, height),
                Opt::Move {
                    regex: _,
                    pos_x,
                    pos_y,
                } => w.move_and_resize(pos_x, pos_y, w.attr().width, w.attr().height),
                Opt::Hide { .. } => w.hide(),
                Opt::Show { .. } => w.show(),
                Opt::Raise { .. } => w.raise(),
            }
        }
    }
}
