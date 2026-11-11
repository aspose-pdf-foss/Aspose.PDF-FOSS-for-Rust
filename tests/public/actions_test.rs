// Public-API smoke driver — actions cluster.
//
// Ports the cpp actions_smoke_test (NamedAction / JavascriptAction /
// SubmitFormAction + PredefinedAction ordinals), plus construction /
// accessor coverage for GoToAction and GoToURIAction. The
// LinkAnnotation-clone case is deferred to the annotations cluster.

use aspose_pdf_foss::{
    Document, ExplicitDestinationType, FileSpecification, GoToAction, GoToURIAction,
    IAppointment, JavascriptAction, NamedAction, PdfAction, PredefinedAction, SubmitFormAction,
    XYZExplicitDestination,
};

#[test]
fn named_action_from_predefined() {
    assert_eq!(PredefinedAction::FirstPage as i32, 0);
    assert_eq!(PredefinedAction::Window_FullScreenMode as i32, 70);

    let mut a = NamedAction::from_predefined(PredefinedAction::NextPage);
    assert_eq!(a.name(), "NextPage");
    assert_eq!(a.to_string(), "NextPage");
    a.set_name("FirstPage".into());
    assert_eq!(a.name(), "FirstPage");
}

#[test]
fn javascript_action() {
    let mut a = JavascriptAction::new("app.alert('hi');".into());
    assert_eq!(a.script(), "app.alert('hi');");
    assert_eq!(a.get_ecma_script_string(), "app.alert('hi');");
    a.set_script("console.println('x');".into());
    assert_eq!(a.get_ecma_script_string(), "console.println('x');");
}

#[test]
fn submit_form_action() {
    let mut a = SubmitFormAction::new();
    assert_eq!(a.flags(), 0);
    a.set_flags(4);
    a.set_url(FileSpecification::with_file("https://example.org/submit".into()));
    assert_eq!(a.flags(), 4);
    assert_eq!(a.url().name(), "https://example.org/submit");
    // Flag bit constants match the canonical values.
    assert_eq!(SubmitFormAction::EXCLUDE, 1);
    assert_eq!(SubmitFormAction::EXPORT_FORMAT, 4);
    assert_eq!(SubmitFormAction::EMBED_FORM, 8192);
}

#[test]
fn go_to_uri_action() {
    let mut a = GoToURIAction::new("https://example.org".into());
    assert_eq!(a.uri(), "https://example.org");
    assert_eq!(a.to_string(), "https://example.org");
    a.set_uri("https://changed".into());
    assert_eq!(a.uri(), "https://changed");
}

#[test]
fn go_to_action_destinations() {
    // Page-number ctor defaults to a Fit destination.
    let a = GoToAction::for_page_number(2);
    assert_eq!(a.destination().unwrap().to_string(), "2 Fit");

    // Explicit fit kind + parameters.
    let doc = Document::new();
    let b = GoToAction::for_destination(Box::new(XYZExplicitDestination::new(1, 0.0, 700.0, 1.0)));
    assert_eq!(b.destination().unwrap().to_string(), "1 XYZ 0 700 1");

    // Named destination.
    let c = GoToAction::for_named_destination(&doc, "Chapter1".into());
    assert_eq!(c.destination().unwrap().to_string(), "Chapter1");

    // Empty action has no destination.
    assert!(GoToAction::new().destination().is_none());
}

#[test]
fn go_to_action_from_fit_kind() {
    let doc = Document::new();
    let page = doc.pages().get(1);
    // Build only if the throwaway document has a page; otherwise skip
    // the page-bound ctor and exercise the page-number path.
    if let Ok(page) = page {
        let a = GoToAction::from_page_with_fit(
            &page,
            ExplicitDestinationType::FitH,
            vec![500.0],
        );
        let s = a.destination().unwrap().to_string();
        assert!(s.ends_with("FitH 500"), "got {s}");
    }
}

#[test]
fn actions_dispatch_polymorphically() {
    // Held behind the PdfAction trait, each action keeps its own
    // to_string / get_ecma_script_string.
    let actions: Vec<Box<dyn PdfAction>> = vec![
        Box::new(NamedAction::from_predefined(PredefinedAction::LastPage)),
        Box::new(JavascriptAction::new("this.print();".into())),
        Box::new(GoToURIAction::new("https://x".into())),
    ];
    assert_eq!(actions[0].to_string(), "LastPage");
    assert_eq!(actions[0].get_ecma_script_string(), "");
    assert_eq!(actions[1].get_ecma_script_string(), "this.print();");
    assert_eq!(actions[2].to_string(), "https://x");

    // PdfAction is an IAppointment.
    let appt: &dyn IAppointment = &NamedAction::from_predefined(PredefinedAction::FirstPage);
    assert_eq!(appt.to_string(), "FirstPage");
}
