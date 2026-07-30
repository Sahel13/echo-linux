use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.sahel.Echo")
        .build();

    let quit_action = gtk::gio::SimpleAction::new("quit", None);
    let quit_application = application.clone();
    quit_action.connect_activate(move |_, _| quit_application.quit());
    application.add_action(&quit_action);

    let window = Rc::new(RefCell::new(None));
    application.connect_activate({
        let window = window.clone();
        move |application| activate(application, &window)
    });
    application.run();
}

fn activate(
    application: &adw::Application,
    existing_window: &Rc<RefCell<Option<adw::ApplicationWindow>>>,
) {
    if existing_window.borrow().is_some() {
        existing_window
            .borrow()
            .as_ref()
            .expect("existing window checked above")
            .present();
        return;
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = gtk::Label::new(Some("Echo"));
    title.add_css_class("title-1");
    title.set_halign(gtk::Align::Start);
    content.append(&title);

    let message = gtk::Label::new(Some("Echo for Linux is ready."));
    message.set_halign(gtk::Align::Start);
    message.set_wrap(true);
    content.append(&message);

    let quit = gtk::Button::with_label("Quit");
    quit.set_halign(gtk::Align::End);
    quit.set_action_name(Some("app.quit"));
    content.append(&quit);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Echo")
        .default_width(360)
        .default_height(180)
        .content(&content)
        .build();

    window.connect_close_request(|window| {
        window.hide();
        gtk::glib::Propagation::Stop
    });
    *existing_window.borrow_mut() = Some(window.clone());
    window.present();
}
