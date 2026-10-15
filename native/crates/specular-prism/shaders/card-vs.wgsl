// Three.js r186 - Node System

// directives


// structs


// uniforms

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform13 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform12 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying2 : vec2<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar47 : mat4x4<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : mat4x4<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec4<f32>;
var<private> v_positionWorld : vec3<f32>;
var<private> positionLocal : vec3<f32>;

// codes


@vertex
fn main( @location( 0 ) uv : vec2<f32>,
	@location( 1 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	varyings.nodeVarying2 = uv;
	nodeVar47 = object.nodeUniform13;
	positionLocal = position;
	v_positionWorld = ( object.nodeUniform13 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	nodeVar48 = ( v_positionWorld - ( object.nodeUniform13 * vec4<f32>( position, 0.0 ) ).xyz );
	nodeVar47[ 3u ][ 0u ] = nodeVar48.x;
	nodeVar47[ 3u ][ 1u ] = nodeVar48.y;
	nodeVar47[ 3u ][ 2u ] = nodeVar48.z;
	nodeVar49 = ( render.cameraViewMatrix * nodeVar47 );
	nodeVar50 = vec3<f32>( length( object.nodeUniform13[ 0u ] ), 0.0, 0.0 );
	nodeVar49[ 0u ][ 0u ] = nodeVar50.x;
	nodeVar49[ 0u ][ 1u ] = nodeVar50.y;
	nodeVar49[ 0u ][ 2u ] = nodeVar50.z;
	nodeVar51 = vec3<f32>( 0.0, length( object.nodeUniform13[ 1u ] ), 0.0 );
	nodeVar49[ 1u ][ 0u ] = nodeVar51.x;
	nodeVar49[ 1u ][ 1u ] = nodeVar51.y;
	nodeVar49[ 1u ][ 2u ] = nodeVar51.z;
	nodeVar49[ 2u ][ 0u ] = vec3<f32>( 0.0, 0.0, 1.0 ).x;
	nodeVar49[ 2u ][ 1u ] = vec3<f32>( 0.0, 0.0, 1.0 ).y;
	nodeVar49[ 2u ][ 2u ] = vec3<f32>( 0.0, 0.0, 1.0 ).z;
	nodeVar52 = ( ( render.cameraProjectionMatrix * nodeVar49 ) * vec4<f32>( position, 1.0 ) );

	// result

	varyings.builtinClipSpace = nodeVar52;

	return varyings;

}
