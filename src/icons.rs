//! Reusable SVG assets embedded in the executable; white strokes accept theme tinting.
use eframe::egui;
use eframe::emath::GuiRounding;

#[derive(Clone, Copy)]
pub enum Icon { Locate, Trash, AddLayer, Move, Microphone, Gear, BackgroundTasks, EditList, Eye, EyeOff, Hand, Play, Pause, Stop, Next, Previous, TrimPlay, MarkStart, MarkEnd, ChevronLeft, ChevronRight, TriangleRight, TriangleDown, Search, Close, Minus, TabGroup, SelectionRect, SelectionEllipse, SelectionLasso, SelectionPolygon, Pencil, Brush, Eraser, Eyedropper, RectangleTool, LineTool, Lock, Unlock, PaintBucket, VectorShape, VectorPath, PathSelection, PathSelectionFilled, VectorTriangle, PenNib, PenFree, PenCurve, PenAdd, PenDelete, PenConvert, Type }

pub(crate) fn paint_layer_button(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32, add: bool) {
    let dpi = ui.ctx().pixels_per_point();
    let side = rect.width().min(rect.height());
    let rect = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
    let pixels = (1.25 * dpi).round().max(1.);
    let phase = if pixels as i32 % 2 == 0 { 0. } else { 0.5 };
    let snap = |value: f32| ((value * dpi - phase).round() + phase) / dpi;
    let p = |x, y| egui::pos2(snap(rect.left() + x * side / 16.), snap(rect.top() + y * side / 16.));
    let stroke = egui::Stroke::new(pixels / dpi, color);
    let painter = ui.painter();
    painter.rect_stroke(egui::Rect::from_min_max(p(2., 2.), p(14., 14.)), 0., stroke, egui::StrokeKind::Middle);
    painter.line_segment([p(5., 8.), p(11., 8.)], stroke);
    if add {
        painter.line_segment([p(8., 5.), p(8., 11.)], stroke);
    }
}

pub fn paint(ui: &egui::Ui, icon: Icon, rect: egui::Rect, color: egui::Color32) {
    if matches!(icon, Icon::AddLayer) {
        paint_layer_button(ui, rect, color, true);
        return;
    }
    if matches!(icon,Icon::Move|Icon::Pencil|Icon::Brush|Icon::Eraser|Icon::Search|Icon::Trash|Icon::Lock|Icon::Unlock|Icon::Eyedropper|Icon::RectangleTool|Icon::LineTool|Icon::PaintBucket|Icon::VectorShape|Icon::VectorPath|Icon::PathSelection|Icon::PathSelectionFilled|Icon::VectorTriangle|Icon::PenNib|Icon::PenFree|Icon::PenCurve|Icon::PenAdd|Icon::PenDelete|Icon::PenConvert|Icon::Type){
        let dpi=ui.ctx().pixels_per_point();let side=rect.width().min(rect.height());let rect=egui::Rect::from_center_size(rect.center(),egui::Vec2::splat(side));
        let pixels=(1.25*dpi).round().max(1.);let width=pixels/dpi;let phase=if pixels as i32%2==0{0.}else{0.5};
        let p=|x:f32,y:f32|{let q=egui::pos2(rect.left()+x/16.*side,rect.top()+y/16.*side);egui::pos2(((q.x*dpi-phase).round()+phase)/dpi,((q.y*dpi-phase).round()+phase)/dpi)};
        let stroke=egui::Stroke::new(width,color);let painter=ui.painter();
        let polyline = |points: &[(f32, f32)]| {
            painter.add(egui::Shape::line(points.iter().map(|&(x, y)| p(x, y)).collect(), stroke));
        };
        match icon{
            Icon::Lock|Icon::Unlock=>{painter.rect_stroke(egui::Rect::from_min_max(p(3.,7.),p(13.,15.)),1./dpi,stroke,egui::StrokeKind::Middle);let shackle=if matches!(icon,Icon::Lock){vec![p(5.,7.),p(5.,4.),p(6.,2.),p(10.,2.),p(11.,4.),p(11.,7.)]}else{vec![p(6.,7.),p(6.,4.),p(7.,2.),p(11.,2.),p(12.,4.),p(12.,5.)]};painter.add(egui::Shape::line(shackle,stroke));painter.circle_filled(p(8.,10.),width*0.8,color);painter.line_segment([p(8.,10.),p(8.,12.)],stroke);},
            Icon::Move=>{painter.line_segment([p(8.,2.),p(8.,14.)],stroke);painter.line_segment([p(2.,8.),p(14.,8.)],stroke);for points in [vec![p(8.,0.5),p(5.,4.),p(11.,4.)],vec![p(8.,15.5),p(5.,12.),p(11.,12.)],vec![p(0.5,8.),p(4.,5.),p(4.,11.)],vec![p(15.5,8.),p(12.,5.),p(12.,11.)]]{painter.add(egui::Shape::convex_polygon(points,color,egui::Stroke::NONE));}},
            Icon::Pencil => {
                polyline(&[(1.5, 14.5), (3., 10.), (11.5, 1.5), (14.5, 4.5), (6., 13.), (1.5, 14.5)]);
                polyline(&[(3., 10.), (6., 13.)]);
                polyline(&[(9.5, 3.5), (12.5, 6.5)]);
            }
            Icon::Brush => {
                painter.add(egui::Shape::convex_polygon(
                    vec![p(6., 8.), p(13.5, 1.), p(15., 2.5), p(8.5, 10.)],
                    color, egui::Stroke::NONE,
                ));
                painter.add(egui::Shape::convex_polygon(
                    vec![p(4.5, 9.5), p(7., 10.5), p(7., 12.5), p(5.5, 14.), p(1., 15.), p(2.5, 12.5), p(3., 10.5)],
                    color, egui::Stroke::NONE,
                ));
            }
            Icon::Eraser=>{painter.add(egui::Shape::closed_line(vec![p(1.,10.),p(9.,2.),p(15.,8.),p(9.,14.),p(5.,14.)],stroke));painter.line_segment([p(5.,6.),p(11.,12.)],stroke);},
            Icon::Search=>{let a=p(1.,1.);let b=p(11.,11.);painter.circle_stroke((a+b.to_vec2())/2.,(b.x-a.x)/2.,stroke);painter.line_segment([p(10.,10.),p(15.,15.)],egui::Stroke::new(width*1.5,color));},
            Icon::Trash=>{painter.line_segment([p(2.,4.),p(14.,4.)],stroke);painter.add(egui::Shape::line(vec![p(6.,3.),p(6.,1.),p(10.,1.),p(10.,3.)],stroke));painter.add(egui::Shape::line(vec![p(4.,6.),p(4.,14.),p(12.,14.),p(12.,6.)],stroke));for x in [7.,9.]{painter.line_segment([p(x,7.),p(x,12.)],stroke);}},
            Icon::Eyedropper => {
                polyline(&[(8., 5.), (12., 1.), (14., 1.), (15., 3.), (11., 7.)]);
                polyline(&[(7., 4.), (12., 9.)]);
                polyline(&[(8., 6.), (2., 12.), (2., 14.), (4., 14.), (10., 8.)]);
                polyline(&[(2., 14.), (1., 15.)]);
            }
            Icon::RectangleTool => {
                painter.rect_stroke(egui::Rect::from_min_max(p(3., 4.), p(13., 12.)), 0., stroke, egui::StrokeKind::Middle);
                polyline(&[(1., 2.), (5., 2.)]);
                polyline(&[(3., 0.5), (3., 3.5)]);
                polyline(&[(11., 14.), (15., 14.)]);
                polyline(&[(13., 12.5), (13., 15.5)]);
            }
            Icon::LineTool => {
                polyline(&[(3., 13.), (13., 3.)]);
                polyline(&[(2., 11.), (2., 14.), (5., 14.)]);
                polyline(&[(11., 2.), (14., 2.), (14., 5.)]);
            }
            Icon::PaintBucket => {
                polyline(&[(5., 2.), (12., 8.), (7., 13.), (5., 13.), (1., 9.), (1., 7.), (5., 2.)]);
                polyline(&[(2., 8.), (12., 8.)]);
                polyline(&[(5., 1.), (8., 6.)]);
                painter.add(egui::Shape::convex_polygon(vec![p(14., 9.), p(16., 13.), p(14., 15.), p(12., 13.)], color, egui::Stroke::NONE));
            }
            Icon::VectorShape => {
                painter.rect_stroke(egui::Rect::from_min_max(p(2., 3.), p(14., 13.)), 0., stroke, egui::StrokeKind::Inside);
            }
            Icon::VectorPath => {
                polyline(&[(2.5, 12.5), (7.5, 3.), (13.5, 9.)]);
                for (x, y) in [(2.5, 12.5), (7.5, 3.), (13.5, 9.)] {
                    painter.rect_filled(egui::Rect::from_center_size(p(x, y), egui::Vec2::splat(width * 2.8)), 0., color);
                }
            }
            Icon::PathSelectionFilled => {
                painter.add(egui::Shape::convex_polygon(
                    vec![p(3., 1.), p(13.5, 9.5), p(8.5, 9.5), p(3., 13.)],
                    color,
                    egui::Stroke::NONE,
                ));
                painter.line_segment([p(7.5, 8.5), p(10., 14.)], egui::Stroke::new(width * 1.8, color));
            }
            Icon::PathSelection => {
                painter.add(egui::Shape::closed_line(
                    vec![p(3., 1.), p(13., 9.), p(9., 9.), p(11., 14.), p(9., 15.), p(7., 10.), p(3., 13.)],
                    stroke,
                ));
            }
            Icon::VectorTriangle => polyline(&[(8., 2.), (14., 13.), (2., 13.), (8., 2.)]),
            Icon::PenConvert => {
                polyline(&[(3., 13.), (7., 3.), (13., 11.)]);
                painter.circle_filled(p(7., 3.), width, color);
            }
            Icon::PenNib | Icon::PenFree | Icon::PenCurve | Icon::PenAdd | Icon::PenDelete => {
                polyline(&[(3., 13.), (5., 6.), (11., 2.), (14., 5.), (10., 11.), (3., 13.)]);
                polyline(&[(3., 13.), (8., 8.)]);
                painter.circle_filled(p(8., 8.), width * 0.85, color);
                polyline(&[(10., 3.), (13., 6.)]);
                match icon {
                    Icon::PenAdd | Icon::PenDelete => {
                        polyline(&[(11., 13.), (16., 13.)]);
                        if matches!(icon, Icon::PenAdd) {
                            polyline(&[(13.5, 10.5), (13.5, 15.5)]);
                        }
                    }
                    Icon::PenFree => polyline(&[(0.5, 9.), (2., 6.), (0.5, 4.), (2., 1.5)]),
                    Icon::PenCurve => polyline(&[(9., 15.5), (12., 15.), (14.5, 13.), (15.5, 10.)]),
                    _ => {}
                }
            }
            Icon::Type => {
                polyline(&[(3., 5.), (3., 2.5), (13., 2.5), (13., 5.)]);
                painter.line_segment([p(8., 2.5), p(8., 13.)], egui::Stroke::new(width * 1.5, color));
                polyline(&[(5.5, 13.), (10.5, 13.)]);
            }
            _=>{}
        }return;
    }
    if matches!(icon,Icon::SelectionRect|Icon::SelectionEllipse|Icon::SelectionLasso|Icon::SelectionPolygon) {
        let p=|x:f32,y:f32|egui::pos2(rect.left()+x/24.*rect.width(),rect.top()+y/24.*rect.height()).round_to_pixels(ui.ctx().pixels_per_point());
        let stroke=egui::Stroke::new(1.7_f32,color);
        let mut points=match icon {
            Icon::SelectionRect=>vec![p(3.,4.),p(21.,4.),p(21.,20.),p(3.,20.)],
            Icon::SelectionEllipse=>(0..40).map(|i|{let a=i as f32*std::f32::consts::TAU/40.;p(12.+10.*a.cos(),12.+8.*a.sin())}).collect(),
            Icon::SelectionPolygon=>vec![p(3.,18.),p(6.,4.),p(21.,8.),p(17.,21.)],
            _=>(0..40).map(|i|{let a=i as f32*std::f32::consts::TAU/40.;p(12.+(9.+a.sin())*a.cos(),10.+7.*a.sin())}).collect(),
        };
        points.push(points[0]);
        ui.painter().add(egui::Shape::line(points,stroke));
        if matches!(icon,Icon::SelectionLasso){ui.painter().add(egui::Shape::line(vec![p(7.,16.),p(6.,21.),p(10.,23.),p(13.,19.)],stroke));}
        return;
    }
    // Keep Next's triangle and bar at their original scale; TrimPlay adds a bar.
    let rect = if matches!(icon, Icon::TrimPlay) {
        egui::Rect::from_center_size(rect.center(), egui::vec2(rect.height()*28.0/24.0, rect.height()))
    } else { rect };
    let source = match icon {
        Icon::Trash => egui::include_image!("../assets/icons/trash.svg"),
        Icon::Locate => egui::include_image!("../assets/icons/locate.svg"),
        Icon::AddLayer => egui::include_image!("../assets/icons/add-layer.svg"),
        Icon::Move => egui::include_image!("../assets/icons/move.svg"),
        Icon::SelectionRect => egui::include_image!("../assets/icons/selection-rect.svg"),
        Icon::SelectionEllipse => egui::include_image!("../assets/icons/selection-ellipse.svg"),
        Icon::SelectionLasso => egui::include_image!("../assets/icons/selection-lasso.svg"),
        Icon::SelectionPolygon => egui::include_image!("../assets/icons/selection-polygon.svg"),
        Icon::Pencil => egui::include_image!("../assets/icons/pencil.svg"),
        Icon::Brush => egui::include_image!("../assets/icons/brush.svg"),
        Icon::Eraser => egui::include_image!("../assets/icons/eraser.svg"),
        Icon::Eyedropper => egui::include_image!("../assets/icons/eyedropper.svg"),
        Icon::Lock => egui::include_image!("../assets/icons/lock.svg"),
        Icon::Unlock => egui::include_image!("../assets/icons/unlock.svg"),
        Icon::LineTool => egui::include_image!("../assets/icons/line-tool.svg"),
        Icon::RectangleTool => egui::include_image!("../assets/icons/rectangle-tool.svg"),
        Icon::PaintBucket => egui::include_image!("../assets/icons/paint-bucket.svg"),
        Icon::VectorShape | Icon::VectorPath | Icon::PathSelection | Icon::PathSelectionFilled | Icon::VectorTriangle | Icon::PenNib | Icon::PenFree | Icon::PenCurve | Icon::PenAdd | Icon::PenDelete | Icon::PenConvert | Icon::Type => unreachable!("painted as paths"),
        Icon::Microphone => egui::include_image!("../assets/icons/microphone.svg"),
        Icon::TabGroup => egui::include_image!("../assets/icons/tab-group.svg"),
        Icon::Minus => egui::include_image!("../assets/icons/minus.svg"),
        Icon::Search => egui::include_image!("../assets/icons/search.svg"),
        Icon::Close => egui::include_image!("../assets/icons/close.svg"),
        Icon::EditList => egui::include_image!("../assets/icons/edit-list.svg"),
        Icon::BackgroundTasks => egui::include_image!("../assets/icons/background-tasks.svg"),
        Icon::Gear => egui::include_image!("../assets/icons/gear.svg"),
        Icon::EyeOff => egui::include_image!("../assets/icons/eye-off.svg"),
        Icon::Eye => egui::include_image!("../assets/icons/eye.svg"),
        Icon::Hand => egui::include_image!("../assets/icons/hand.svg"),
        Icon::Play => egui::include_image!("../assets/icons/play.svg"),
        Icon::ChevronLeft => egui::include_image!("../assets/icons/chevron-left.svg"),
        Icon::ChevronRight => egui::include_image!("../assets/icons/chevron-right.svg"),
        Icon::Next | Icon::Previous => egui::include_image!("../assets/icons/next.svg"),
        Icon::TrimPlay => egui::include_image!("../assets/icons/trim-play.svg"),
        Icon::MarkStart => egui::include_image!("../assets/icons/mark-start.svg"),
        Icon::MarkEnd => egui::include_image!("../assets/icons/mark-end.svg"),
        Icon::Stop => egui::include_image!("../assets/icons/stop.svg"),
        Icon::Pause => egui::include_image!("../assets/icons/pause.svg"),
        Icon::TriangleRight => egui::include_image!("../assets/icons/triangle-right.svg"),
        Icon::TriangleDown => egui::include_image!("../assets/icons/triangle-down.svg"),
    };
    let mut image=egui::Image::new(source).fit_to_exact_size(rect.size()).tint(color);
    if matches!(icon,Icon::Previous) {image=image.uv(egui::Rect::from_min_max(egui::pos2(1.0,0.0),egui::pos2(0.0,1.0)));}
    image.paint_at(ui,rect);
}

/// Square vector-icon button shared by compact toolbar controls.
pub fn button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    let response = ui.add_sized([28.0, 28.0], egui::Button::new(""));
    paint(ui, icon, response.rect.shrink(7.0), ui.visuals().text_color());
    response.on_hover_text(tooltip)
}
