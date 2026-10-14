// Three.js r186 - Node System

// directives


// structs


// uniforms


// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar14 : vec4<f32>;

// codes


@vertex
fn main( @builtin( vertex_index ) vertexIndex : u32,
	@location( 0 ) uv : vec2<f32> ) -> VaryingsStruct {

	// flow
	// code

	varyings.nodeVarying0 = uv;
	nodeVar14 = vec4<f32>( array< f32, 3 >( -1.0, -1.0, 3.0 )[ vertexIndex ], array< f32, 3 >( 3.0, -1.0, -1.0 )[ vertexIndex ], 0.0, 1.0 );

	// result

	varyings.builtinClipSpace = nodeVar14;

	return varyings;

}
