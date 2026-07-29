use adw::prelude::*;

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.sahel.Echo")
        .build();

    application.connect_activate(build_window);
    application.run();
}

fn build_window(application: &adw::Application) {
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

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Echo")
        .default_width(360)
        .default_height(180)
        .content(&content)
        .build();
    window.present();
}
