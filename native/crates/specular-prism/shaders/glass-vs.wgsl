// Three.js r186 - Node System

// directives


// structs


// uniforms

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : mat4x4<f32>,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform9 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform16 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : vec3<f32>,
	nodeUniform19 : f32,
	nodeUniform20 : vec3<f32>,
	nodeUniform21 : f32,
	nodeUniform22 : f32,
	nodeUniform24 : f32,
	nodeUniform25 : vec3<f32>,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform29 : f32,
	nodeUniform30 : f32,
	nodeUniform36 : f32,
	nodeUniform37 : mat4x4<f32>,
	nodeUniform39 : f32,
	nodeUniform40 : f32,
	nodeUniform42 : f32,
	nodeUniform43 : f32,
	nodeUniform44 : f32,
	nodeUniform46 : mat4x4<f32>,
	nodeUniform47 : f32,
	nodeUniform48 : f32,
	nodeUniform49 : f32,
	nodeUniform50 : f32,
	nodeUniform51 : f32,
	nodeUniform52 : f32,
	nodeUniform54 : f32,
	nodeUniform55 : f32,
	nodeUniform56 : f32,
	nodeUniform60 : f32,
	nodeUniform61 : vec3<f32>,
	nodeUniform62 : f32,
	nodeUniform63 : f32,
	nodeUniform64 : vec3<f32>,
	nodeUniform65 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform23 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform34 : vec3<f32>,
	nodeUniform35 : vec3<f32>,
	nodeUniform32 : vec3<f32>,
	nodeUniform33 : vec3<f32>,
	cameraWorldMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform28 : vec3<f32>,
	nodeUniform58 : vec2<f32>,
	nodeUniform53 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_normalWorldGeometry : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> normalLocal : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> VERTEX_nodeVar267 : vec4<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) normal : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	varyings.v_positionWorld = ( object.nodeUniform4 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	normalLocal = normal;
	nodeVar1 = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform7 * normalLocal ), 0.0 ) ).xyz );
	varyings.v_normalViewGeometry = nodeVar1;
	normalViewGeometry = normalize( varyings.v_normalViewGeometry );
	varyings.v_normalWorldGeometry = normalize( ( vec4<f32>( normalViewGeometry, 0.0 ) * render.cameraViewMatrix ).xyz );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform4 );
	varyings.v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	VERTEX_nodeVar267 = ( render.cameraProjectionMatrix * vec4<f32>( varyings.v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar267;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
