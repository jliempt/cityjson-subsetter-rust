#![allow(warnings)]

use core::str::Bytes;
use std::fs::File;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{self, Visitor, MapAccess, DeserializeSeed, SeqAccess};
use serde_json::{Value, json};
use std::fmt;
use std::marker::PhantomData;
use std::collections::BTreeMap;
use lazy_static::lazy_static;
use std::sync::{ Mutex, MutexGuard};

static mut BBOX: [ f32; 4 ] = [ 0.0, 0.0, 0.0, 0.0 ];
static mut TRANSFORM: Value = json!( null );
lazy_static! {
    static ref VERTICES: Mutex< BTreeMap< u32, ( [ f32; 3 ], u32 ) > > = Mutex::new( BTreeMap::new() );
}


#[derive(Serialize, Deserialize, Default, Debug)]
pub struct CityJSON {

	r#type: Value,
    version: Value,
    #[serde(deserialize_with = "deserialize_cityobjects")]
    CityObjects: Value,
/*    #[serde(deserialize_with = "deserialize_vertices")]
    vertices: Vec< u32 >,*/
    extensions: Option< Value >,
    metadata: Option< Value >,
    transform: Option< Value >,
    appearance: Option< Value >,
    geometry_templates: Option< Value >,
    #[serde(skip_deserializing)]
    vertices: Value

}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Vertices {

	#[serde(deserialize_with = "deserialize_vertices")]
	vertices: BTreeMap< u32, [ u32; 3 ] >,

}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Transform {

	transform: Option< Value >,

}


pub fn select_cos( buf: &Vec< u8 >, file_out: &File, bbox: [ f32; 4 ] ) -> CityJSON {

    unsafe {
        BBOX = bbox;
    }

	let mut out: CityJSON = serde_json::from_slice( buf ).expect("Error parsing CityJSON buffer");

    let mut vertices = VERTICES.lock().unwrap();
    let mut vertices_out: Vec< [ f32; 3 ] > = Vec::with_capacity( vertices.len() );


    for ( k, v ) in vertices.iter() {

        vertices_out.push( v.0 );

    }

    out.vertices = serde_json::value::to_value( vertices_out ).unwrap();

    out

}

pub fn select_vertices( buf: &Vec< u8 >, bbox: [ f32; 4 ] ) {

	let mut bbox_copy = bbox;

	unsafe {

		if !TRANSFORM.is_null() {

			bbox_copy[ 0 ] = ( ( bbox_copy[ 0 ] as f64 - TRANSFORM[ "translate" ][ 0 ].as_f64().unwrap() ) / TRANSFORM[ "scale" ][ 0 ].as_f64().unwrap() ) as f32 ;
			bbox_copy[ 1 ] = ( ( bbox_copy[ 1 ] as f64 - TRANSFORM[ "translate" ][ 1 ].as_f64().unwrap() ) / TRANSFORM[ "scale" ][ 1 ].as_f64().unwrap() ) as f32 ;
			bbox_copy[ 2 ] = ( ( bbox_copy[ 2 ] as f64 - TRANSFORM[ "translate" ][ 0 ].as_f64().unwrap() ) / TRANSFORM[ "scale" ][ 0 ].as_f64().unwrap() ) as f32 ;
			bbox_copy[ 3 ] = ( ( bbox_copy[ 3 ] as f64 - TRANSFORM[ "translate" ][ 1 ].as_f64().unwrap() ) / TRANSFORM[ "scale" ][ 1 ].as_f64().unwrap() ) as f32 ;

		}

		BBOX = bbox_copy;

	}

	println!("{:?}", bbox_copy);

	let mut out: Vertices = serde_json::from_slice( buf ).expect("Error parsing CityJSON buffer");

}

pub fn get_transform( buf: &Vec< u8 > ) {

	let out: Transform = serde_json::from_slice( buf ).expect("Error parsing CityJSON buffer");

	if out.transform != None {

		unsafe {
			TRANSFORM = out.transform.unwrap();
		}

	}

}

fn get_centroid( geometry: &Value, vertices: &MutexGuard< BTreeMap< u32, ( [ f32; 3 ], u32 ) > > ) -> Option< [ f32; 2 ] > {

    fn recursionvisit( a: &Value, vs: &mut Vec< u32 >, vertices: &MutexGuard< BTreeMap< u32, ( [ f32; 3 ], u32 ) > > ) {

        if a.is_array() {

            for n in ( 0..a.as_array().unwrap().len() ) {

                if a.is_array() {

                    recursionvisit( &a[ n ], vs, vertices )

                } else {

                    let index: u32 = a.as_u64().unwrap() as u32;
                    vs.push( index );

/*                    match vertices.get( & index ) {

                        Some( vertex ) => vs.push( *vertex ),
                        None => {}

                    }*/

                }

            }

        } else {

            let index: u32 = a.as_u64().unwrap() as u32;
            vs.push( index );

/*            match vertices.get( & index ) {

                Some( vertex ) => vs.push( *vertex ),
                None => vs.push(  ),

            }*/

        }

    }

    let mut centroid: [ f32; 2 ] = [ 0.0, 0.0 ];
    let mut total: u32 = 0;

    let geoms = geometry.as_array().unwrap();

    for i in ( 0..geoms.len() ) {

        let geom = &geoms[ i ];
        let mut vs: Vec< u32 > = Vec::new();

        recursionvisit( &geom[ "boundaries" ], &mut vs, vertices );

        for j in ( 0..vs.len() ) {

            match vertices.get( &vs[ j ] ) {

                Some( vertex ) => {

                    total += 1;
                    centroid[ 0 ] += vertex.0[ 0 ] as f32;
                    centroid[ 1 ] += vertex.0[ 1 ] as f32;

                },
                None => {}

            }

        }

        // TODO: transform, store COs for which centroid could not be determined

    }

    if total != 0 {

        centroid[ 0 ] /= total as f32;
        centroid[ 1 ] /= total as f32;

        println!("{:?}", centroid);

        Some( centroid )

    } else {

        None

    }

}

fn update_array_indices( a: &mut Value, vertices: &MutexGuard< BTreeMap< u32, ( [ f32; 3 ], u32 ) > > ) {

	if a.is_array() {

        for n in ( 0..a.as_array().unwrap().len() ) {

        	update_array_indices( &mut a[ n ], vertices );

        }

    } else if !a.is_null() {

    	let index = vertices.get( &(a.as_u64().unwrap() as u32) ).unwrap().1;

    	*a = serde_json::value::to_value( index ).unwrap();

    }


}


fn deserialize_cityobjects< 'de, D >( deserializer: D ) -> Result< Value, D::Error >
where

    D: Deserializer< 'de >,

{

    struct COVisitor;

    impl< 'de > Visitor< 'de > for COVisitor
    {
        /// Return type of this visitor
        type Value = Value;

        // Error message if data that is not of this type is encountered while deserializing
        fn expecting( &self, formatter: &mut fmt::Formatter ) -> fmt::Result {
            formatter.write_str("a key/value entry")
        }

        // Traverse CityObjects
        fn visit_map<S>( self, mut map: S ) -> Result< Value, S::Error >
        where
            S: MapAccess<'de>,
        {

            // TODO: adding parent-children

            let vertices = VERTICES.lock().unwrap();
            let mut bbox: [ f32; 4 ];
            unsafe {
                bbox = BBOX;
            }
            let mut res: Vec< Value > = Vec::new(); 

            // Iterate over keys and values in "CityObjects"
            while let Some( ( key, mut value ) ) = map.next_entry::< String, serde_json::Value >()? {

                
                let centroid = get_centroid( &value[ "geometry" ], &vertices );

                match centroid {

                    Some( c ) => {

                        if c[ 0 ] >= bbox[ 0 ] && c[ 1 ] >= bbox[ 1 ]
                            && c[ 0 ] < bbox[ 2 ] && c[ 1 ] < bbox[ 3 ] {

                            	let mut geoms = value[ "geometry" ].as_array_mut().unwrap();

							    for i in ( 0..geoms.len() ) {

							        let mut geom = geoms.get_mut( i ).unwrap();

							        update_array_indices( &mut geom.get_mut( "boundaries" ).unwrap(), &vertices );

							    }

                                // Add CO to result if centroid within bbox
                                res.push( value );

                            }

                    },
                    None => {},

                }


            }

            Ok( serde_json::value::to_value( res ).unwrap() )

        }
    }

    deserializer.deserialize_map(COVisitor)

}


fn deserialize_vertices< 'de, D >( deserializer: D ) -> Result< BTreeMap< u32, [ u32; 3] >, D::Error >
where

    D: Deserializer< 'de >,

{

    struct VertexVisitor;

    impl< 'de > Visitor< 'de > for VertexVisitor
    {
        /// Return type of this visitor
        type Value = BTreeMap< u32, [ u32; 3] >;

        // Error message if data that is not of this type is encountered while deserializing
        fn expecting( &self, formatter: &mut fmt::Formatter ) -> fmt::Result {
            formatter.write_str("an array")
        }

        // Traverse CityObjects
        fn visit_seq<S>( self, mut seq: S ) -> Result< BTreeMap< u32, [ u32; 3] >, S::Error >
        where
            S: SeqAccess<'de>,
        {

            let mut bbox: [ f32; 4 ];
            unsafe {
                bbox = BBOX;
            }

            println!("{:?}", bbox);
            // 5% bbox margin
            bbox[ 0 ] = bbox[ 0 ] * 0.95;
            bbox[ 1 ] = bbox[ 1 ] * 0.95;
            bbox[ 2 ] = bbox[ 2 ] * 1.05;
            bbox[ 3 ] = bbox[ 3 ] * 1.05;

            let mut res = BTreeMap::new();
            let mut i: u32 = 0;
            let mut vertices = VERTICES.lock().unwrap();

            // Amount of vertices stored, used later for index
    		let mut j = 0;

        	while let Some( elem ) = seq.next_element::< [ f32; 3 ] >()? {

/*        		println!("{:?}", elem);
        		println!("{:?}", bbox);
*/
        		if elem[ 0 ] >= bbox[ 0 ] && elem[ 1 ] >= bbox[ 1 ]
                    && elem[ 0 ] < bbox[ 2 ] && elem[ 1 ] < bbox[ 3 ] {

                    	// println!("{:?}", elem);
                    	// println!("{:?}", bbox);
                    	// println!("ja");

                        vertices.insert( i, ( elem, j ) );

                        j += 1;

                    }

                i += 1;

            }

            Ok( res )

        }
    }

    // Create the visitor and ask the deserializer to drive it. The
    // deserializer will call visitor.visit_map() if a map is present in
    // the input data.

    deserializer.deserialize_seq(VertexVisitor)

}