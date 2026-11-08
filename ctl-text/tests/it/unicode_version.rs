//! The Unicode versions behind widths and cluster boundaries are pinned, so
//! an upgrade of either data set is a reviewed change.

use ctl_text::UNICODE_VERSION;

const UPGRADE: &str = "a dependency upgrade moved a Unicode version: re-read the width and cluster tests, then update the pin";

#[test]
fn the_width_data_is_on_the_pinned_version() {
    assert_eq!(UNICODE_VERSION.width, (17, 0, 0), "{UPGRADE}");
}

#[test]
fn the_segmentation_data_is_on_the_pinned_version() {
    assert_eq!(UNICODE_VERSION.segmentation, (17, 0, 0), "{UPGRADE}");
}
