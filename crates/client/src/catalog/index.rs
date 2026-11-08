use std::fmt::Write;

use super::Entry;

const GUIDE: &str = "# Catalog

This folder holds rendered images of the objects, the materials, and the scenes of the game. \
A program makes the images and this file. Do not change them by hand.

Each entry sets its camera, pose, game time, seed, and light to fixed values. \
The same program on the same computer writes the same bytes each time.

## Make the images

| Task | Command |
| --- | --- |
| Render all entries | `cargo run --release -p voidwright-client --bin catalog` |
| Render some entries | `cargo run --release -p voidwright-client --bin catalog -- wanderer material-island` |

The program renders the entries with names that start with one of the given words. \
With no words, it renders all entries and removes the images that no entry uses. \
It always writes this file again.

## Add an entry

1. Open `crates/client/src/catalog/entries.rs`.
2. Add an `Entry::new` item to the list of its group, for example `wanderer_views()` or `world_views()`. \
Give a name, a description, a subject, and a camera. \
For a new group, add a function with its list, and add one line for it in `entries()`.
3. Render the new entry with the command above.
4. Commit the new image and this file.

The name is also the file name of the image. Use lowercase letters, digits, and hyphens.

| Subject | Image |
| --- | --- |
| `Subject::Wanderer { gait, time }` | The wanderer on the stage of the model inspector. The coat moves for 2 seconds before `time`. The lantern lights the figure. |
| `Subject::Swatch(material)` | A ball and a flat patch of the material in the light of Vael. |
| `Subject::Ground(color)` | A ball and a flat patch of an island color. The light is baked into the color, as on the island. |
| `Subject::Surface(kind)` | A grey patch with the detail surface, close. |
| `Subject::World { seed, time, walk }` | The game world as the game draws it, at the game time. `walk` is a tile offset that the player walks to. |

The camera turns around a target. \
`Camera::new([width, height])` sets the size of the image in pixels. \
`.at(focus, offset)` sets the target: `Focus::Origin`, `Focus::Player`, or `Focus::Bone(bone)`, \
and an offset in meters. \
`.orbit(yaw, pitch, distance)` sets the angles in radians and the distance in meters. \
The angles are the same as the angles of `--orbit`.

Each material constant (`const NAME: Material` at the top level of a module) must have an entry. \
A test finds the constants in the source code. \
Add a new constant to the `MATERIALS` list of its module. \
For a new module, add a `materials(...)` line to `entries()`.

For a new kind of object, add a `Subject` kind and its draw step in `crates/client/src/catalog/app.rs`.

Keep each image to 640 × 640 pixels or fewer in total, and keep the folder below 15 MB. \
Tests check these limits.

## Entries

| Name | Shows | Image |
| --- | --- | --- |
";

#[must_use]
pub fn index(entries: &[Entry]) -> String {
    let mut text = GUIDE.to_owned();
    for entry in entries {
        let width = entry.camera.size[0] / 2;
        let (name, about, file) = (&entry.name, &entry.about, entry.file());
        writeln!(
            text,
            "| `{name}` | {about} | <img src=\"{file}\" width=\"{width}\" alt=\"{name}\"> |"
        )
        .expect("a string takes the text");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::index;
    use crate::catalog::entries;

    #[test]
    fn the_index_lists_every_entry_with_its_image() {
        let entries = entries();
        let text = index(&entries);
        for entry in &entries {
            let row = format!(
                "| `{}` | {} | <img src=\"{}.png\"",
                entry.name, entry.about, entry.name
            );
            assert!(text.contains(&row), "{row}");
        }
        let rows = text
            .lines()
            .filter(|line| line.contains("<img src="))
            .count();
        assert_eq!(rows, entries.len());
    }

    #[test]
    fn the_index_tells_how_to_render_and_add_an_entry() {
        let text = index(&[]);
        assert!(text.contains("`cargo run --release -p voidwright-client --bin catalog`"));
        assert!(text.contains("crates/client/src/catalog/entries.rs"));
        assert!(text.ends_with("| --- | --- | --- |\n"));
    }
}
