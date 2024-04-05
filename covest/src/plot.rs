use plotters::prelude::*;
use plotters::style::full_palette::*;
use plotters::style::{BLACK, WHITE};

use crate::trader::Trader;

pub struct Plot {
    pub trader: Trader,
    pub out_file: String,
    pub min_x: u64,
    pub max_x: u64,
    pub min_y: f64,
    pub max_y: f64,
}

impl Plot {
    // pub fn new(
    //     trader: Trader,
    //     out_file: String,
    //     min_x: u64,
    //     max_x: u64,
    //     min_y: f64,
    //     max_y: f64,
    // ) -> Self {
    //     Self {
    //         trader,
    //         out_file,
    //         min_x,
    //         max_x,
    //         min_y,
    //         max_y,
    //     }
    // }
    //
    // pub fn root(&self) -> anyhow::Result<DrawingArea<BitMapBackend, Shift>> {
    //     let root = BitMapBackend::new(&self.out_file.clone(), (2048, 1024)).into_drawing_area();
    //     root.fill(&WHITE)?;
    //     Ok(root)
    // }
    //
    // pub fn chart(
    //     &self,
    // ) -> anyhow::Result<ChartContext<BitMapBackend, Cartesian2d<RangedCoordu64, RangedCoordf64>>>
    // {
    //     let root = self.root()?;
    //     let chart = ChartBuilder::on(&root)
    //         .margin_top(20)
    //         .margin_bottom(20)
    //         .margin_left(30)
    //         .margin_right(30)
    //         .set_all_label_area_size(130)
    //         .caption(
    //             format!("{} Trade History", self.trader.key),
    //             ("sans-serif", 40.0).into_font(),
    //         )
    //         .build_cartesian_2d(self.min_x..self.max_x, self.min_y..self.max_y)?;
    //     Ok(chart)
    // }
    //
    // pub fn mesh(
    //     &self,
    // ) -> anyhow::Result<ChartContext<BitMapBackend, Cartesian2d<RangedCoordu64, RangedCoordf64>>>
    // {
    //     let root = self.root()?;
    //     let mut chart = self.chart()?;
    //     chart
    //         .configure_mesh()
    //         .light_line_style(WHITE)
    //         .label_style(("sans-serif", 30, &BLACK).into_text_style(&root))
    //         .x_desc("Slot")
    //         .y_desc("PnL")
    //         .draw()?;
    //     Ok(chart)
    // }

    pub fn random_color() -> RGBAColor {
        let colors = [
            RED_A400,
            BLUEGREY_700,
            GREY_400,
            GREY_900,
            BROWN_700,
            DEEPORANGE_A200,
            DEEPORANGE_200,
            ORANGE_A200,
            AMBER_300,
            AMBER_800,
            YELLOW_A400,
            YELLOW_600,
            LIME_400,
            LIME_800,
            LIGHTGREEN_700,
            GREEN_500,
            GREEN_900,
            TEAL_700,
            TEAL_200,
            CYAN_800,
            LIGHTBLUE_A200,
            BLUE_A700,
            BLUE_400,
            BLUE_800,
            INDIGO_800,
            INDIGO_300,
            DEEPPURPLE_A100,
            PURPLE_A400,
            PURPLE_200,
            PINK_600,
            RED_800,
            RED_200,
            BLACK,
            WHITE,
        ];
        // get random color
        RGBAColor::from(colors[rand::random::<usize>() % colors.len()])
    }
}
