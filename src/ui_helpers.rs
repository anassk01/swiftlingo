use gtk::prelude::*;
use gtk::{
    Box as GtkBox,
    Button,
    Entry,
    Frame,
    Label,
    Orientation,
    Widget,
};
use gtk::glib;

/// Helper for creating a labeled widget with consistent layout
#[allow(dead_code)]
pub fn create_labeled_widget<W: IsA<Widget>>(
    label_text: &str,
    widget: &W,
    align_start: bool,
) -> GtkBox {
    let box_widget = GtkBox::new(Orientation::Horizontal, 10);
    
    let label = Label::new(Some(label_text));
    if align_start {
        label.set_halign(gtk::Align::Start);
    }
    label.set_hexpand(true);
    
    box_widget.append(&label);
    box_widget.append(widget);
    
    box_widget
}

/// Helper for creating a consistent frame with a title and content
#[allow(dead_code)]
pub fn create_frame<W: IsA<Widget>>(title: &str, child: &W) -> Frame {
    let frame = Frame::new(Some(title));
    frame.set_margin_start(16);
    frame.set_margin_end(16);
    frame.set_margin_bottom(16);
    
    frame.set_child(Some(child));
    
    frame
}

/// Creates a box with consistent margins
#[allow(dead_code)]
pub fn create_padded_box(orientation: Orientation, spacing: i32) -> GtkBox {
    let box_widget = GtkBox::new(orientation, spacing);
    box_widget.set_margin_start(16);
    box_widget.set_margin_end(16);
    box_widget.set_margin_top(16);
    box_widget.set_margin_bottom(16);
    
    box_widget
}

/// Creates a button with consistent styling
pub fn create_button(label: &str, is_primary: bool, is_destructive: bool) -> Button {
    let button = Button::with_label(label);
    
    if is_primary {
        button.add_css_class("suggested-action");
    }
    
    if is_destructive {
        button.add_css_class("destructive-action");
    }
    
    button
}

/// Wraps a callback with UI state variables to reduce cloning boilerplate
#[allow(dead_code)]
pub struct CallbackData<T, F>
where
    F: Fn(&T) + 'static,
{
    data: T,
    callback: F,
}

#[allow(dead_code)]
impl<T, F> CallbackData<T, F>
where
    F: Fn(&T) + 'static,
{
    pub fn new(data: T, callback: F) -> Self {
        CallbackData { data, callback }
    }
    
    pub fn call(&self) {
        (self.callback)(&self.data);
    }
}

/// Helper to create a button with a callback that uses shared data without cloning
#[allow(dead_code)]
pub fn connect_button<T: 'static, F: Fn(&T) + 'static>(
    button: &Button,
    data: T,
    callback: F,
) {
    let data = std::rc::Rc::new(CallbackData::new(data, callback));
    button.connect_clicked(move |_| {
        data.call();
    });
}

/// Helper to show a message dialog
#[allow(dead_code)]
pub fn show_message_dialog(
    parent: Option<&impl IsA<gtk::Window>>,
    message_type: gtk::MessageType,
    buttons_type: gtk::ButtonsType,
    message: &str,
) -> gtk::MessageDialog {
    let dialog = gtk::MessageDialog::new(
        parent,
        gtk::DialogFlags::MODAL,
        message_type,
        buttons_type,
        message,
    );
    
    dialog.connect_response(|dialog, _| {
        dialog.destroy();
    });
    
    dialog.show();
    dialog
}

/// Helper to run a task in the background and update UI when done
pub fn spawn_local_task<F, Fut>(task: F)
where
    F: FnOnce() -> Fut + 'static,
    Fut: std::future::Future<Output = ()> + 'static,
{
    let context = glib::MainContext::default();
    context.spawn_local(async move {
        task().await;
    });
}

/// Helper to create a standard form field
#[allow(dead_code)]
pub fn create_form_field(label_text: &str, placeholder: Option<&str>) -> (GtkBox, Entry) {
    let field_box = GtkBox::new(Orientation::Horizontal, 10);
    
    let label = Label::new(Some(label_text));
    label.set_halign(gtk::Align::Start);
    label.set_width_chars(15);
    
    let entry = Entry::new();
    entry.set_hexpand(true);
    
    if let Some(placeholder_text) = placeholder {
        entry.set_placeholder_text(Some(placeholder_text));
    }
    
    field_box.append(&label);
    field_box.append(&entry);
    
    (field_box, entry)
}