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

static mut BBOX: [ u32; 4 ] = [ 0, 0, 0, 0 ];
// static mut VERTICES: BTreeMap< u32, [ u32; 3 ] > = BTreeMap::new();

lazy_static! {
    static ref VERTICES: Mutex< BTreeMap< u32, [ u32; 3 ] > > = Mutex::new( BTreeMap::new() );
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


pub fn select_cos( buf: &Vec< u8 >, file_out: &File, bbox: [ u32; 4 ] ) -> CityJSON {

    unsafe {
        BBOX = bbox;
    }

	let mut out: CityJSON = serde_json::from_slice( buf ).expect("Error parsing CityJSON buffer");

    let mut vertices = VERTICES.lock().unwrap();
    let mut vertices_out: Vec< [ u32; 3 ] > = Vec::with_capacity( vertices.len() );

    for ( k, v ) in vertices.iter() {

        vertices_out.push( *v );

    }

    out.vertices = serde_json::value::to_value( vertices_out ).unwrap();

    out

}

pub fn select_vertices( buf: &Vec< u8 >, bbox: [ u32; 4 ] ) {

    unsafe {
        BBOX = bbox;
    }

	let mut out: Vertices = serde_json::from_slice( buf ).expect("Error parsing CityJSON buffer");

}

fn get_centroid( geometry: &Value, vertices: &MutexGuard< BTreeMap< u32, [ u32; 3 ] > > ) -> Option< [ f32; 2 ] > {

    fn recursionvisit( a: &Value, vs: &mut Vec< u32 >, vertices: &MutexGuard< BTreeMap< u32, [ u32; 3 ] > > ) {

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
                    centroid[ 0 ] += vertex[ 0 ] as f32;
                    centroid[ 1 ] += vertex[ 1 ] as f32;

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
            let mut bbox: [ u32; 4 ];
            unsafe {
                bbox = BBOX;
            }
            let mut res: Vec< Value > = Vec::new(); 

            // Iterate over keys and values in "CityObjects"
            while let Some( ( key, value ) ) = map.next_entry::< String, serde_json::Value >()? {

                
                let centroid = get_centroid( &value[ "geometry" ], &vertices );

                match centroid {

                    Some( c ) => {

                        if c[ 0 ] >= bbox[ 0 ] as f32 && c[ 1 ] >= bbox[ 1 ] as f32
                            && c[ 0 ] < bbox[ 2 ] as f32 && c[ 1 ] < bbox[ 3 ] as f32 {

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

            let mut bbox: [ u32; 4 ];
            unsafe {
                bbox = BBOX;
            }

            // 5% bbox margin
            bbox[ 0 ] = ( bbox[ 0 ] as f32 * 0.95 ) as u32;
            bbox[ 1 ] = ( bbox[ 1 ] as f32 * 0.95 ) as u32;
            bbox[ 2 ] = ( bbox[ 2 ] as f32 * 1.05 ) as u32;
            bbox[ 3 ] = ( bbox[ 3 ] as f32 * 1.05 ) as u32;

            let mut res = BTreeMap::new();
            let mut i: u32 = 0;
            let mut vertices = VERTICES.lock().unwrap();

        	while let Some( elem ) = seq.next_element::< [ u32; 3 ] >()? {

        		if elem[ 0 ] >= bbox[ 0 ] && elem[ 1 ] >= bbox[ 1 ]
                    && elem[ 0 ] < bbox[ 2 ] && elem[ 1 ] < bbox[ 3 ] {

                        vertices.insert( i, elem );

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