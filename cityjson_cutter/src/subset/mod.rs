mod subset_bbox;

use crate::subset::subset_bbox::CityJSON;
use std::fs::File;

pub fn get_subset_bbox( buf: Vec< u8 >, file_out: &File, bbox: [ u32; 4 ] ) -> CityJSON {

	subset_bbox::select_vertices( &buf, bbox );

	let out = subset_bbox::select_cos( &buf, file_out, bbox );

	out

}