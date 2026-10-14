// Three.js r186 - Node System

// directives


// structs


// uniforms

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform20 : vec3<f32>,
	nodeUniform21 : vec3<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform19 : vec3<f32>,
	cameraWorldMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform45 : vec2<f32>,
	nodeUniform40 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : vec3<f32>,
	nodeUniform8 : f32,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32,
	nodeUniform12 : mat4x4<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform16 : f32,
	nodeUniform17 : f32,
	nodeUniform22 : f32,
	nodeUniform23 : mat4x4<f32>,
	nodeUniform25 : f32,
	nodeUniform26 : f32,
	nodeUniform28 : f32,
	nodeUniform29 : f32,
	nodeUniform30 : f32,
	nodeUniform32 : mat4x4<f32>,
	nodeUniform33 : f32,
	nodeUniform34 : f32,
	nodeUniform35 : f32,
	nodeUniform36 : f32,
	nodeUniform37 : f32,
	nodeUniform38 : f32,
	nodeUniform39 : f32,
	nodeUniform41 : f32,
	nodeUniform42 : f32,
	nodeUniform43 : f32,
	nodeUniform47 : f32,
	nodeUniform48 : vec3<f32>,
	nodeUniform49 : f32,
	nodeUniform50 : f32,
	nodeUniform51 : vec3<f32>,
	nodeUniform52 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_normalWorldGeometry : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> normalLocal : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> VERTEX_nodeVar289 : vec4<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec3<f32>;
var<private> positionLocal : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) normal : vec3<f32>,
	@location( 1 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	normalLocal = normal;
	nodeVar0 = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform5 * normalLocal ), 0.0 ) ).xyz );
	varyings.v_normalViewGeometry = nodeVar0;
	positionLocal = position;
	varyings.v_positionWorld = ( object.nodeUniform12 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	normalViewGeometry = normalize( varyings.v_normalViewGeometry );
	varyings.v_normalWorldGeometry = normalize( ( vec4<f32>( normalViewGeometry, 0.0 ) * render.cameraViewMatrix ).xyz );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform12 );
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	VERTEX_nodeVar289 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar289;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
