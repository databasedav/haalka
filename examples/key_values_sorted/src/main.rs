use bevy::prelude::*;
use haalka::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin,
        ))
        .add_systems(Startup, (ui_root, camera))
        .run();
}

static PAIRS: Lazy<MutableVec<(Mutable<String>, Mutable<String>)>> = Lazy::new(|| {
    [("the", "quick"), ("brown", "fox"), ("jumps", "over"), ("the", "lazy")]
        .into_iter()
        .map(|(a, b)| (Mutable::new(a.to_string()), Mutable::new(b.to_string())))
        .collect::<Vec<_>>()
        .into()
});

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .with_style(|style| {
            style.width = Val::Percent(100.);
            style.height = Val::Percent(100.);
        })
        .align_content(Align::center())
        // .child(Column::new().items_signal_vec(PAIRS.signal_vec_cloned().map(callback)))
        .spawn(world);
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
