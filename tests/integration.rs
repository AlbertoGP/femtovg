
use gpucanvas::{
    Canvas,
    Path,
    Paint,
    Color,
    Winding,
    FillRule,
    renderer::Void
};

#[test]
fn path_with_single_move_to() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(10.0, 10.0);
    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn path_with_two_lines() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.line_to(10.0, 10.0);
    path.line_to(10.0, 10.0);
    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn path_with_close_points() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(10.0, 10.0);
    path.line_to(10.0001, 10.0);
    path.line_to(10.0001, 10.000001);
    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn path_with_points_at_limits() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(10.0, 10.0);
    path.line_to(std::f32::MAX, std::f32::MAX);
    path.quad_to(10.0, 10.0, -std::f32::MAX, std::f32::MAX);
    path.bezier_to(10.0, 10.0, std::f32::MAX, 5000.0, -std::f32::MAX, -std::f32::MAX);
    path.rounded_rect_varying(
        -std::f32::MAX, -std::f32::MAX,
        std::f32::MAX, std::f32::MAX,
        std::f32::MAX, std::f32::MAX,
        std::f32::MAX, std::f32::MAX
    );
    path.close();

    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn path_with_points_around_zero() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(0.0, 0.0);
    path.line_to(0.0, 0.0);
    path.line_to(0.0001, 0.0000003);
    path.quad_to(0.002, 0.0001, -0.002, 0.0001);
    path.bezier_to(0.0001, 0.002, -0.002, 0.0001, -0.002, 0.0001);
    path.rounded_rect_varying(
        -std::f32::MAX, -std::f32::MAX,
        std::f32::MAX, std::f32::MAX,
        std::f32::MAX, 0.0001,
        0.0001, 0.0001
    );

    path.close();

    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn degenerate_stroke() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(0.5, 0.5);
    path.line_to(2., 2.);
    path.line_to(2., 2.);
    path.line_to(4., 2.);
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn degenerate_arc_to() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(10.0, 10.0);
    path.arc_to(10.0, 10.0001, 10.0, 10.0001, 2.0);
    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn degenerate_arc() {
    let mut canvas = Canvas::new(Void).unwrap();

    let mut path = Path::new();
    path.move_to(10.0, 10.0);
    path.arc(10.0, 10.0, 10.0, 0.0, std::f32::MAX, Winding::CW);

    canvas.fill_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
    canvas.stroke_path(&mut path, Paint::color(Color::rgb(100, 100, 100)));
}

#[test]
fn path_contains_point() {
    let mut canvas = Canvas::new(Void).unwrap();
    
    // Star - cancave & self crossing
    let mut path = Path::new();
    path.move_to(50.0, 0.0);
    path.line_to(21.0, 90.0);
    path.line_to(98.0, 35.0);
    path.line_to(2.0, 35.0);
    path.line_to(79.0, 90.0);
    path.close();
    
    // Center of the star should be hollow for even-odd rule
    assert!(!canvas.contains_point(&mut path, 50.0, 45.0, FillRule::EvenOdd));
    assert!(canvas.contains_point(&mut path, 50.0, 5.0, FillRule::EvenOdd));
    
    // Center of the star should be fill for NonZero rule
    assert!(canvas.contains_point(&mut path, 50.0, 45.0, FillRule::NonZero));
    assert!(canvas.contains_point(&mut path, 50.0, 5.0, FillRule::NonZero));
}

#[test]
fn text_location_respects_scale() {
    let mut canvas = Canvas::new(Void).unwrap();
    
    canvas.add_font("examples/assets/Roboto-Regular.ttf");
    
    let mut paint = Paint::color(Color::black());
    canvas.scale(5.0, 5.0);

    let bounds = canvas.text_bounds(100.0, 100.0, "Hello", paint);
    assert_eq!(bounds[0], 100.0);
    assert_eq!(bounds[1], 84.0);
    
    let bounds = canvas.fill_text(100.0, 100.0, "Hello", paint);
    assert_eq!(bounds[0], 100.0);
    assert_eq!(bounds[1], 84.0);
}
