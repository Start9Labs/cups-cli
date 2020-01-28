use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use failure::Error;
use futures::poll;
use pancurses::*;

use cupslib::{Creds, Message, Pubkey, UserData};

pub struct Windows {
    main: Window,
    sidebar: Window,
    topbar: Window,
    feed: Window,
    input: Window,
}

pub async fn tui(creds: Creds) -> Result<(), Error> {
    let main = initscr();
    let sidebar = main
        .subwin(main.get_max_y(), main.get_max_x() / 4, 0, 0)
        .unwrap();
    let topbar = main
        .subwin(
            main.get_max_y() / 8,
            main.get_max_x() * 3 / 4,
            0,
            main.get_max_x() / 4,
        )
        .unwrap();
    let feed = main
        .subwin(
            main.get_max_y() * 5 / 8,
            main.get_max_x() * 3 / 4,
            main.get_max_y() / 8,
            main.get_max_x() / 4,
        )
        .unwrap();
    let input = main
        .subwin(
            main.get_max_y() / 4,
            main.get_max_x() * 3 / 4,
            main.get_max_y() * 3 / 4,
            main.get_max_x() / 4,
        )
        .unwrap();
    let creds = Arc::new(creds);
    let res = tui_inner(
        Arc::new(Windows {
            main,
            sidebar,
            topbar,
            feed,
            input,
        }),
        creds,
    )
    .await;
    endwin();
    res
}

#[derive(Clone, Copy, Debug)]
pub enum Selected {
    Sidebar,
    Input,
    Feed,
    Namebar,
}
impl Selected {
    pub fn rotate(&mut self) {
        match *self {
            Selected::Sidebar => {
                *self = Selected::Input;
            }
            Selected::Input => {
                *self = Selected::Feed;
            }
            Selected::Feed => {
                *self = Selected::Namebar;
            }
            Selected::Namebar => {
                *self = Selected::Sidebar;
            }
        }
    }
}

pub struct State<
    U: Fn() -> UF,
    UF: Future<Output = Result<Vec<UserData>, Error>>,
    M: Fn(Pubkey) -> MF,
    MF: Future<Output = Result<Vec<Message>, Error>>,
> {
    window: Arc<Windows>,
    pub user_data: Vec<UserData>,
    user_future: Option<Pin<Box<UF>>>,
    user_fn: U,
    pub messages: Vec<Message>,
    message_future: Option<Pin<Box<MF>>>,
    message_fn: M,
    pub selected: usize,
}
impl<U, UF, M, MF> State<U, UF, M, MF>
where
    U: Fn() -> UF,
    UF: Future<Output = Result<Vec<UserData>, Error>>,
    M: Fn(Pubkey) -> MF,
    MF: Future<Output = Result<Vec<Message>, Error>>,
{
    pub async fn new(window: Arc<Windows>, u: U, m: M) -> Result<Self, Error> {
        Ok(State {
            window,
            user_data: u().await?,
            user_future: Some(Box::pin(u())),
            user_fn: u,
            messages: Vec::new(),
            message_future: None,
            message_fn: m,
            selected: 0,
        })
    }
    pub async fn update(&mut self) -> Result<(), Error> {
        use std::task::Poll;
        if let Some(mut uf) = self.user_future.take() {
            match poll!(uf.as_mut()) {
                Poll::Ready(s) => {
                    self.user_future = Some(Box::pin((self.user_fn)()));
                    let clear = self.user_data.len();
                    self.user_data = s?;
                    render_sidebar(&*self.window, &self.user_data, clear, self.selected);
                }
                Poll::Pending => self.user_future = Some(uf),
            }
        } else {
            self.user_future = Some(Box::pin((self.user_fn)()));
        }
        if let Some(user) = self.user_data.get(self.selected) {
            if let Some(mut mf) = self.message_future.take() {
                match poll!(mf.as_mut()) {
                    Poll::Ready(s) => {
                        self.message_future = Some(Box::pin((self.message_fn)(Pubkey(user.id))));
                        self.messages = s?;
                        render_feed(&*self.window, &self.messages);
                    }
                    Poll::Pending => self.message_future = Some(mf),
                }
            } else {
                self.message_future = Some(Box::pin((self.message_fn)(Pubkey(user.id))));
            }
        }
        Ok(())
    }
}

async fn tui_inner(win: Arc<Windows>, creds: Arc<Creds>) -> Result<(), Error> {
    let ucreds = creds.clone();
    let mcreds = creds.clone();
    let mut state = State::new(
        win.clone(),
        move || cupslib::fetch_users(ucreds.clone()),
        move |id| cupslib::fetch_messages(mcreds.clone(), id, None),
    )
    .await?;
    init(&*win);
    let mut selected = Selected::Sidebar;
    loop {
        match win.main.getch() {
            Some(Input::KeyResize) | Some(Input::KeyAbort) | Some(Input::Character('q')) => break,
            Some(Input::KeySTab) | Some(Input::Character('\t')) => {
                hide_selection(&*win, selected);
                selected.rotate();
                show_selection(&*win, selected);
            }
            Some(Input::KeyUp) => match selected {
                Selected::Sidebar => {
                    if state.selected > 0 {
                        state.selected -= 1;
                        change_sidebar_selection(
                            &*win,
                            &state.user_data,
                            state.selected,
                            state.selected + 1,
                        )
                    }
                }
                _ => (),
            },
            _ => (),
        }
        state.update().await?;
    }
    Ok(())
}

fn init(win: &Windows) {
    resize_term(0, 0);
    curs_set(0);
    noecho();
    win.main.clear();
    win.main.border('|', '|', '-', '-', '+', '+', '+', '+');
    win.sidebar.attron(Attribute::Bold);
    win.sidebar.border('|', '|', '-', '-', '+', '+', '+', '+');
    win.sidebar.attroff(Attribute::Bold);
    win.topbar.border('|', '|', '-', '-', '+', '+', '+', '+');
    win.feed.border('|', '|', '-', '-', '+', '+', '+', '+');
    win.input.border('|', '|', '-', '-', '+', '+', '+', '+');
    win.main.refresh();
}

fn show_selection(win: &Windows, sel: Selected) {
    use Selected::*;
    match sel {
        Sidebar => {
            win.sidebar.attron(Attribute::Bold);
            win.sidebar.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.sidebar.attroff(Attribute::Bold);
            win.sidebar.refresh();
        }
        Namebar => {
            win.topbar.attron(Attribute::Bold);
            win.topbar.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.topbar.attroff(Attribute::Bold);
            win.topbar.refresh();
        }
        Feed => {
            win.feed.attron(Attribute::Bold);
            win.feed.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.feed.attroff(Attribute::Bold);
            win.feed.refresh();
        }
        Input => {
            win.input.attron(Attribute::Bold);
            win.input.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.input.attroff(Attribute::Bold);
            win.input.refresh();
        }
    }
    win.main.refresh();
}

fn hide_selection(win: &Windows, sel: Selected) {
    use Selected::*;
    match sel {
        Sidebar => {
            win.sidebar.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.sidebar.refresh();
        }
        Namebar => {
            win.topbar.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.topbar.refresh();
        }
        Feed => {
            win.feed.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.feed.refresh();
        }
        Input => {
            win.input.border('|', '|', '-', '-', '+', '+', '+', '+');
            win.input.refresh();
        }
    }
}

fn render_sidebar(win: &Windows, data: &[UserData], clear: usize, selected: usize) {
    if data.len() < clear {
        for i in data.len()..clear {
            win.sidebar.mv(i as i32 + 4, 2);
            win.sidebar.clrtoeol();
        }
    }
    for (i, user) in data.iter().enumerate() {
        win.sidebar.mv(i as i32 + 2, 2);
        if i == selected {
            win.sidebar.attron(Attribute::Reverse);
        }
        win.sidebar.clrtoeol();
        if let Some(name) = &user.name {
            win.sidebar.addstr(&format!(
                " - {:30} -> {}",
                &name[..std::cmp::min(name.len(), 30)],
                user.unreads
            ));
        } else {
            win.sidebar.addstr(&format!(
                "- {}... -> {}",
                base32::encode(base32::Alphabet::RFC4648 { padding: false }, &user.id)[..27]
                    .to_lowercase(),
                user.unreads
            ));
        }
        if i == selected {
            win.sidebar.attroff(Attribute::Reverse);
        }
    }
}

fn change_sidebar_selection(win: &Windows, data: &[UserData], selected: usize, clear: usize) {}

fn render_feed(win: &Windows, msgs: &[Message]) {}
