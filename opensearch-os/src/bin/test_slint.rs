fn main() {
    slint::slint! {
        export component MainWindow inherits Window {
            Text { text: "Hello World"; }
        }
    }
    let ui = MainWindow::new().unwrap();
    // Do not show the window
    println!("Before run_event_loop");
    slint::run_event_loop().unwrap();
    println!("After run_event_loop");
}
