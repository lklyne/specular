// The composite, folded into the downscale to the output size.
//
// For Prism every paint branch of the web build's composite weighs 0, so it
// is `m = smoothstep(threshold, threshold + softness, a)` and
// `vec4(clamp(rgb, 0, 1) * m, m)`, premultiplied. Each output pixel averages
// a `factor`-square block of display texels, so a supersampled render comes
// out antialiased. The values are written as they are: the look is tuned
// for them being read as sRGB-encoded.

struct Params {
	threshold : f32,
	softness : f32,
	factor : u32,
	straight : u32,
};

@group( 0 ) @binding( 0 ) var display : texture_2d<f32>;
@group( 0 ) @binding( 1 ) var<uniform> params : Params;

@vertex
fn vs_main( @builtin( vertex_index ) index : u32 ) -> @builtin( position ) vec4<f32> {
	let x = array<f32, 3>( -1.0, 3.0, -1.0 )[ index ];
	let y = array<f32, 3>( -1.0, -1.0, 3.0 )[ index ];
	return vec4<f32>( x, y, 0.0, 1.0 );
}

@fragment
fn fs_main( @builtin( position ) position : vec4<f32> ) -> @location( 0 ) vec4<f32> {
	let base = vec2<u32>( floor( position.xy ) ) * params.factor;
	var sum = vec4<f32>( 0.0 );
	for ( var y = 0u; y < params.factor; y++ ) {
		for ( var x = 0u; x < params.factor; x++ ) {
			let d = textureLoad( display, base + vec2<u32>( x, y ), 0 );
			let m = smoothstep( params.threshold, params.threshold + params.softness, d.a );
			sum += vec4<f32>( clamp( d.rgb, vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ) * m, m );
		}
	}
	var out = sum / f32( params.factor * params.factor );
	if ( params.straight != 0u && out.a > 0.0 ) {
		out = vec4<f32>( out.rgb / out.a, out.a );
	}
	return out;
}
