//! Small module with display related functions.

use crate::log::RunLog;
use std::fs::File;
use std::io::prelude::*;
use std::io::Error;
use std::iter::repeat;
use std::iter::repeat_with;
use std::path::Path;

/// all svg colors names used for histograms displays
pub const HISTOGRAM_COLORS: [&str; 6] = ["red", "blue", "green", "yellow", "purple", "brown"];

pub(crate) type Point = (f64, f64);

/// all graphics elements for one `RunLog` display.
pub struct Scene {
    /// Each task is an animated rectangle.
    /// We also display a black rectangle underneath.
    /// idle times are also displayed as animated rectangles.
    pub rectangles: Vec<Rectangle>,
    /// Dependencies are shown as segments.
    pub segments: Vec<(Point, Point)>,
    /// We can hover on tasks to get more information.
    /// rectangles[i] is associated to labels[i].
    pub labels: Vec<String>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            rectangles: Vec::new(),
            segments: Vec::new(),
            labels: Vec::new(),
        }
    }
}

/// colors used for each thread
pub(crate) const COLORS: [[f32; 3]; 8] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 1.0, 0.0],
    [1.0, 0.0, 1.0],
    [0.0, 1.0, 1.0],
    [0.5, 0.5, 0.5],
    [1.0, 0.5, 0.5],
];

/// Tasks are animated as a set of rectangles.
pub struct Rectangle {
    /// color (rgb+alpha)
    pub color: [f32; 3],
    /// opacity
    pub opacity: f32,
    /// x coordinate
    pub x: f64,
    /// y coordinate
    pub y: f64,
    /// width
    pub width: f64,
    /// height
    pub height: f64,
    /// when animation starts and ends (if any)
    pub animation: Option<(u64, u64)>,
}

impl Rectangle {
    /// Creates a new rectangle
    pub fn new(
        color: [f32; 3],
        opacity: f32,
        position: (f64, f64),
        sizes: (f64, f64),
        animation: Option<(u64, u64)>,
    ) -> Rectangle {
        Rectangle {
            color,
            opacity,
            x: position.0,
            y: position.1,
            width: sizes.0,
            height: sizes.1,
            animation,
        }
    }
}

/// saves a set of rectangles and edges as an animated svg file.
/// 1 animated second is 1 milli second of run.
pub(crate) fn write_svg_file<P: AsRef<Path>>(scene: &Scene, path: P) -> Result<(), Error> {
    let mut file = File::create(path)?;
    fill_svg_file(scene, &mut file)
}

/// fill given file with a set of rectangles and edges as an animated svg.
pub(crate) fn fill_svg_file(scene: &Scene, file: &mut File) -> Result<(), Error> {
    let svg_width = 1920; // this is just an aspect ratio
    let svg_height = 1080;

    let xmax = scene
        .rectangles
        .iter()
        .map(|r| r.width + r.x)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let ymax = scene
        .rectangles
        .iter()
        .map(|r| r.height + r.y)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let xmin = scene
        .rectangles
        .iter()
        .map(|r| r.x)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let ymin = scene
        .rectangles
        .iter()
        .map(|r| r.y)
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let xscale = f64::from(svg_width) / (xmax - xmin);
    let yscale = f64::from(svg_height) / (ymax - ymin);

    let random_id: usize = rand::random();

    // Header
    writeln!(
        file,
        "<?xml version=\"1.0\"?>
<svg viewBox=\"0 0 {} {}\" version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">",
        svg_width, svg_height,
    )?;
    // we start by edges so they will end up below tasks
    for (start, end) in &scene.segments {
        write!(
            file,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"2.0\"/>",
            (start.0 - xmin) * xscale,
            (start.1 - ymin) * yscale,
            (end.0 - xmin) * xscale,
            (end.1 - xmin) * yscale
        )?;
    }
    let min_time = scene
        .rectangles
        .iter()
        .filter_map(|r| r.animation.map(|t| t.0))
        .min()
        .unwrap();
    let max_time = scene
        .rectangles
        .iter()
        .filter_map(|r| r.animation.map(|t| t.1))
        .max()
        .unwrap();
    let total_time = max_time - min_time;

    for rectangle in &scene.rectangles {
        if let Some(animation) = rectangle.animation {
            // first a black rectangle
            writeln!(
                file,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"black\"/>",
                (rectangle.x - xmin) * xscale,
                (rectangle.y - ymin) * yscale,
                rectangle.width * xscale,
                rectangle.height * yscale,
            )?;

            // now the animated one
            let (start_time, end_time) = animation;
            writeln!(file,
            "<rect class=\"task{}\" x=\"{}\" y=\"{}\" width=\"0\" height=\"{}\" fill=\"rgba({},{},{},{})\">
<animate attributeType=\"XML\" attributeName=\"width\" from=\"0\" to=\"{}\" begin=\"{}s\" dur=\"{}s\" fill=\"freeze\"/>
</rect>",
        random_id,
        (rectangle.x-xmin)*xscale,
        (rectangle.y-ymin)*yscale,
        rectangle.height*yscale,
        (rectangle.color[0] * 255.0) as u32,
        (rectangle.color[1] * 255.0) as u32,
        (rectangle.color[2] * 255.0) as u32,
        rectangle.opacity,
        rectangle.width*xscale,
        ((start_time-min_time)*60) as f64 / total_time as f64,
        ((end_time - start_time)*60) as f64 / total_time as f64,
        )?;
        } else {
            writeln!(
                file,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\"/>",
                (rectangle.x - xmin) * xscale,
                (rectangle.y - ymin) * yscale,
                rectangle.width * xscale,
                rectangle.height * yscale,
                (rectangle.color[0] * 255.0) as u32,
                (rectangle.color[1] * 255.0) as u32,
                (rectangle.color[2] * 255.0) as u32,
                rectangle.opacity,
            )?;
        }
    }

    for (index, (rectangle, label)) in scene
        .rectangles
        .iter()
        .filter(|&r| r.animation.is_some())
        .zip(scene.labels.iter())
        .enumerate()
    {
        // now the box for the tooltip
        writeln!(file, "<g id=\"tip_{}_{}\">", random_id, index)?;
        let x = (rectangle.x - xmin) * xscale;
        let mut y = (rectangle.y - ymin) * yscale + 40.0;
        if y > (svg_height / 2) as f64 {
            y -= 160.0; // so that label does end below image
        }
        let height = label.lines().count() as f64 * 20.0;
        writeln!(
            file,
            "<rect x=\"{}\" y=\"{}\" width=\"300\" height=\"{}\" fill=\"white\" stroke=\"black\"/>",
            x,
            y,
            height + 10.0
        )?;
        for line in label.lines() {
            y += 20.0;
            writeln!(file, "<text x=\"{}\" y=\"{}\">{}</text>", x + 5.0, y, line)?;
        }
        writeln!(file, "</g>")?;
    }

    // this part will allow to get more info on tasks by hovering over them
    write!(
        file,
        "
   <style>
      .task-highlight {{
        fill: #ec008c;
        opacity: 1;
      }}
    </style>
  <script><![CDATA[

    var tasks = document.getElementsByClassName('task{}');

    for (var i = 0; i < tasks.length; i++) {{
      var tip = document.getElementById('tip_{}_'+i);
      tip.style.display='none';
      tasks[i].tip = tip;
      tasks[i].addEventListener('mouseover', mouseOverEffect);
      tasks[i].addEventListener('mouseout', mouseOutEffect);
    }}

    function mouseOverEffect() {{
      this.classList.add(\"task-highlight\");
      this.tip.style.display='block';
    }}

    function mouseOutEffect() {{
      this.classList.remove(\"task-highlight\");
      this.tip.style.display='none';
    }}
  ]]></script>
",
        random_id, random_id,
    )?;

    write!(file, "</svg>")?;
    Ok(())
}

/// Display histogram for given logs set inside html file.
pub(crate) fn histogram(
    file: &mut File,
    logs: &[Vec<RunLog>],
    bars_number: usize,
) -> Result<(), Error> {
    let min_duration = logs
        .iter()
        .map(|l| l.first().map(|fl| fl.duration).unwrap())
        .min()
        .unwrap();
    let max_duration = logs
        .iter()
        .map(|l| l.last().map(|ll| ll.duration).unwrap())
        .max()
        .unwrap();

    // lets compute how many durations go in each bar
    let mut bars: Vec<Vec<usize>> = repeat_with(|| repeat(0).take(bars_number).collect())
        .take(logs.len())
        .collect();
    let slot = (max_duration - min_duration) / bars_number as u64;
    for (algorithm, algorithm_logs) in logs.iter().enumerate() {
        for duration in algorithm_logs.iter().map(|l| l.duration) {
            let mut index = if slot == 0 {
                0 // if there is only one duration it's not really a histogram
                  // but display it nonetheless
            } else {
                ((duration - min_duration) / slot) as usize
            };
            if index == bars_number {
                index -= 1;
            }
            bars[algorithm][index] += 1;
        }
    }

    // now, just draw one rectangle for each bar
    let width = 1920;
    let height = 1080;
    write!(file, "<svg viewBox=\"0 0 {} {}\">", width, height)?;
    write!(
        file,
        "<rect width=\"{}\" height=\"{}\" fill=\"white\"/>",
        width, height
    )?;
    let max_count = bars.iter().flat_map(|b| b.iter()).max().unwrap();
    let unit_height = (height - 100) as f32 / *max_count as f32;
    let unit_width = width as f32 / bars_number as f32;
    let algorithms_number = logs.len() as f32;
    for (algorithm_index, (counts, color)) in
        bars.iter().zip(HISTOGRAM_COLORS.iter().cycle()).enumerate()
    {
        for (index, &count) in counts.iter().enumerate() {
            if count != 0 {
                write!(
                    file,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    algorithm_index as f32 * unit_width / algorithms_number
                        + unit_width * index as f32,
                    (height - 100) as f32 - (count as f32 * unit_height),
                    unit_width / algorithms_number,
                    count as f32 * unit_height,
                    color
                )?;
            }
        }
    }
    write!(
        file,
        "<text x=\"{}\" y=\"{}\">{}</text>",
        width / 2,
        height - 50,
        (min_duration + max_duration) / 2
    )?;
    write!(
        file,
        "<text x=\"100\" y=\"{}\">{}</text>",
        height - 50,
        min_duration
    )?;
    write!(
        file,
        "<text x=\"{}\" y=\"{}\">{}</text>",
        width - 100,
        height - 50,
        max_duration
    )?;
    write!(file, "</svg>")?;
    Ok(())
}
