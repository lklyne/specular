// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : vec3<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : f32;

// codes
fn fn1 ( color : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;


	if ( ( color.w == 0.0 ) ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	} else {

		nodeVar0 = vec4<f32>( ( color.xyz / vec3<f32>( color.w ) ), color.w );

	}


	return nodeVar0;

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar1 = nodeVar0;
	nodeVar2 = clamp( nodeVar1.w, 0.0, 1.0 );
	nodeVar3 = ( nodeVar1.xyz / vec3<f32>( max( nodeVar2, 0.0001 ) ) );
	nodeVar4 = ( nodeVar3 * vec3<f32>( object.nodeUniform2 ) );
	nodeVar5 = ( ( ( pow( max( ( nodeVar4 + vec3<f32>( ( object.nodeUniform3 * ( 1.0 - smoothstep( 0.0, 0.35, dot( nodeVar4, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ) ) ) ) ), vec3<f32>( 0.0 ) ), vec3<f32>( ( 1.0 / object.nodeUniform4 ) ) ) - vec3<f32>( 0.5 ) ) * vec3<f32>( object.nodeUniform5 ) ) + vec3<f32>( 0.5 ) );
	nodeVar6 = mix( vec3<f32>( dot( nodeVar5, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), nodeVar5, object.nodeUniform6 );
	nodeVar7 = max( max( nodeVar6.x, nodeVar6.y ), nodeVar6.z );
	nodeVar8 = mix( vec3<f32>( dot( nodeVar6, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), nodeVar6, ( 1.0 + ( object.nodeUniform7 * ( 1.0 - ( ( nodeVar7 - min( min( nodeVar6.x, nodeVar6.y ), nodeVar6.z ) ) / max( nodeVar7, 0.0001 ) ) ) ) ) );
	nodeVar9 = ( object.nodeUniform8 * 0.2 );
	nodeVar10 = ( object.nodeUniform9 * 0.2 );
	nodeVar11 = vec4<f32>( ( mix( nodeVar3, max( vec3<f32>( ( nodeVar8.x * ( ( 1.0 + nodeVar9 ) + nodeVar10 ) ), ( nodeVar8.y * ( 1.0 - nodeVar10 ) ), ( nodeVar8.z * ( ( 1.0 - nodeVar9 ) + nodeVar10 ) ) ), vec3<f32>( 0.0 ) ), object.nodeUniform10 ) * vec3<f32>( nodeVar2 ) ), nodeVar2 );
	nodeVar12 = fn0( fn1( vec4<f32>( nodeVar11.xyz, clamp( nodeVar11.w, 0.0, 1.0 ) ) ) );
	nodeVar13 = clamp( nodeVar12.w, 0.0, 1.0 );

	// result

	output.color = vec4<f32>( mix( object.nodeUniform11, ( nodeVar12.xyz / vec3<f32>( max( nodeVar13, 0.0001 ) ) ), nodeVar13 ), nodeVar13 );

	return output;

}
