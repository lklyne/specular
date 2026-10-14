// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct NodeBuffer_1401Struct {
	value : array< vec4<f32>, 8 >
};
@binding( 1 ) @group( 1 )
var<uniform> NodeBuffer_1401 : NodeBuffer_1401Struct;

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

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec2<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : vec2<f32>;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : vec2<f32>;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec2<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec2<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar40 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec4<f32>;

// codes
fn mx_rotl32 ( x : u32, k : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : u32;

	nodeVar0 = k;
	nodeVar1 = x;

	return ( ( nodeVar1 << u32( nodeVar0 ) ) | ( nodeVar1 >> u32( ( 32 - nodeVar0 ) ) ) );

}


fn mx_bjfinal ( a : u32, b : u32, c : u32 ) -> u32 {

	var nodeVar0 : u32;
	var nodeVar1 : u32;
	var nodeVar2 : u32;

	nodeVar0 = c;
	nodeVar1 = b;
	nodeVar2 = a;
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 14 ) );
	nodeVar2 = ( nodeVar2 ^ nodeVar0 );
	nodeVar2 = ( nodeVar2 - mx_rotl32( nodeVar0, 11 ) );
	nodeVar1 = ( nodeVar1 ^ nodeVar2 );
	nodeVar1 = ( nodeVar1 - mx_rotl32( nodeVar2, 25 ) );
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 16 ) );
	nodeVar2 = ( nodeVar2 ^ nodeVar0 );
	nodeVar2 = ( nodeVar2 - mx_rotl32( nodeVar0, 4 ) );
	nodeVar1 = ( nodeVar1 ^ nodeVar2 );
	nodeVar1 = ( nodeVar1 - mx_rotl32( nodeVar2, 14 ) );
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 24 ) );

	return nodeVar0;

}


fn mx_select ( b : bool, t : f32, f : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : bool;
	var nodeVar3 : f32;

	nodeVar0 = f;
	nodeVar1 = t;
	nodeVar2 = b;

	return select( nodeVar0, nodeVar1, nodeVar2 );

}


fn mx_negate_if ( val : f32, b : bool ) -> f32 {

	var nodeVar0 : bool;
	var nodeVar1 : f32;
	var nodeVar2 : f32;

	nodeVar0 = b;
	nodeVar1 = val;

	return select( nodeVar1, ( - nodeVar1 ), nodeVar0 );

}


fn mx_floor ( x : f32 ) -> i32 {

	var nodeVar0 : f32;

	nodeVar0 = x;

	return i32( floor( nodeVar0 ) );

}


fn mx_fade ( t : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = t;

	return ( ( ( nodeVar0 * nodeVar0 ) * nodeVar0 ) * ( ( nodeVar0 * ( ( nodeVar0 * 6.0 ) - 15.0 ) ) + 10.0 ) );

}


fn mx_hash_int_1 ( x : i32, y : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : u32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : u32;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = 2u;
	nodeVar3 = 0u;
	nodeVar4 = 0u;
	nodeVar5 = 0u;
	nodeVar5 = ( ( 3735928559u + ( nodeVar2 << 2u ) ) + 13u );
	nodeVar4 = nodeVar5;
	nodeVar3 = nodeVar4;
	nodeVar3 = ( nodeVar3 + u32( nodeVar1 ) );
	nodeVar4 = ( nodeVar4 + u32( nodeVar0 ) );

	return mx_bjfinal( nodeVar3, nodeVar4, nodeVar5 );

}


fn mx_gradient_float_0 ( hash : u32, x : f32, y : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : u32;
	var nodeVar3 : u32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = hash;
	nodeVar3 = ( nodeVar2 & 7u );
	nodeVar4 = mx_select( ( nodeVar3 < 4u ), nodeVar1, nodeVar0 );
	nodeVar5 = ( 2.0 * mx_select( ( nodeVar3 < 4u ), nodeVar0, nodeVar1 ) );

	return ( mx_negate_if( nodeVar4, bool( ( nodeVar3 & 1u ) ) ) + mx_negate_if( nodeVar5, bool( ( nodeVar3 & 2u ) ) ) );

}


fn mx_bilerp_0 ( v0 : f32, v1 : f32, v2 : f32, v3 : f32, s : f32, t : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;

	nodeVar0 = t;
	nodeVar1 = s;
	nodeVar2 = v3;
	nodeVar3 = v2;
	nodeVar4 = v1;
	nodeVar5 = v0;
	nodeVar6 = ( 1.0 - nodeVar1 );

	return ( ( ( 1.0 - nodeVar0 ) * ( ( nodeVar5 * nodeVar6 ) + ( nodeVar4 * nodeVar1 ) ) ) + ( nodeVar0 * ( ( nodeVar3 * nodeVar6 ) + ( nodeVar2 * nodeVar1 ) ) ) );

}


fn mx_gradient_scale2d_0 ( v : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = v;

	return ( 0.6616 * nodeVar0 );

}


fn mx_perlin_noise_float_0 ( p : vec2<f32> ) -> f32 {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;

	nodeVar0 = p;
	nodeVar1 = 0;
	nodeVar2 = 0;
	nodeVar3 = nodeVar0.x;
	nodeVar1 = mx_floor( nodeVar3 );
	nodeVar4 = ( nodeVar3 - f32( nodeVar1 ) );
	nodeVar5 = nodeVar0.y;
	nodeVar2 = mx_floor( nodeVar5 );
	nodeVar6 = ( nodeVar5 - f32( nodeVar2 ) );
	nodeVar7 = mx_fade( nodeVar4 );
	nodeVar8 = mx_fade( nodeVar6 );
	nodeVar9 = mx_bilerp_0( mx_gradient_float_0( mx_hash_int_1( nodeVar1, nodeVar2 ), nodeVar4, nodeVar6 ), mx_gradient_float_0( mx_hash_int_1( ( nodeVar1 + 1 ), nodeVar2 ), ( nodeVar4 - 1.0 ), nodeVar6 ), mx_gradient_float_0( mx_hash_int_1( nodeVar1, ( nodeVar2 + 1 ) ), nodeVar4, ( nodeVar6 - 1.0 ) ), mx_gradient_float_0( mx_hash_int_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ) ), ( nodeVar4 - 1.0 ), ( nodeVar6 - 1.0 ) ), nodeVar7, nodeVar8 );

	return mx_gradient_scale2d_0( nodeVar9 );

}




@fragment
fn main( @location( 0 ) nodeVarying2 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec2<f32>( 0.0, 0.0 );
	nodeVar1 = nodeVarying2;
	nodeVar2 = nodeVar1;

	for ( var i : i32 = 0; i < 2; i ++ ) {

		nodeVar3 = smoothstep( 0.0, 1.0, length( ( nodeVar1 - vec2<f32>( 0.5 ) ) ) );
		nodeVar4 = ( 1.0 - nodeVar3 );
		nodeVar5 = ( f32( i ) + 1.0 );
		nodeVar6 = ( 0.5 * ( object.nodeUniform1 + 41.5 ) );
		nodeVar2.x = ( nodeVar2.x + ( ( ( ( object.nodeUniform0 * nodeVar4 ) / nodeVar5 ) * sin( ( nodeVar6 + ( ( nodeVar5 * 0.4 ) * smoothstep( 0.0, 1.0, nodeVar2.y ) ) ) ) ) * cos( ( ( nodeVar6 * 0.2 ) + ( ( nodeVar5 * 2.4 ) * smoothstep( 0.0, 1.0, nodeVar2.y ) ) ) ) ) );
		nodeVar2.y = ( nodeVar2.y + ( ( ( object.nodeUniform0 * nodeVar4 ) / nodeVar5 ) * cos( ( nodeVar6 + ( ( nodeVar5 * 2.0 ) * smoothstep( 0.0, 1.0, nodeVar2.x ) ) ) ) ) );

	}

	nodeVar7 = ( nodeVar2 - vec2<f32>( 0.5 ) );
	nodeVar8 = ( - ( ( 3.0 * object.nodeUniform2 ) * nodeVar3 ) );
	nodeVar9 = cos( nodeVar8 );
	nodeVar10 = sin( nodeVar8 );
	nodeVar7 = vec2<f32>( ( ( nodeVar7.x * nodeVar9 ) - ( nodeVar7.y * nodeVar10 ) ), ( ( nodeVar7.x * nodeVar10 ) + ( nodeVar7.y * nodeVar9 ) ) );
	nodeVar7 = ( nodeVar7 + vec2<f32>( 0.5 ) );
	nodeVar0 = nodeVar7;
	nodeVar11 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar12 = 0.0;

	for ( var i : i32 = 0; i < i32( object.nodeUniform3 ); i ++ ) {

		nodeVar13 = pow( max( ( ( ( 0.4122214708 * NodeBuffer_1401.value[ i ].xyz.x ) + ( 0.5363325363 * NodeBuffer_1401.value[ i ].xyz.y ) ) + ( 0.0514459929 * NodeBuffer_1401.value[ i ].xyz.z ) ), 0.0 ), 0.3333333333333333 );
		nodeVar14 = pow( max( ( ( ( 0.2119034982 * NodeBuffer_1401.value[ i ].xyz.x ) + ( 0.6806995451 * NodeBuffer_1401.value[ i ].xyz.y ) ) + ( 0.1073969566 * NodeBuffer_1401.value[ i ].xyz.z ) ), 0.0 ), 0.3333333333333333 );
		nodeVar15 = pow( max( ( ( ( 0.0883024619 * NodeBuffer_1401.value[ i ].xyz.x ) + ( 0.2817188376 * NodeBuffer_1401.value[ i ].xyz.y ) ) + ( 0.6299787005 * NodeBuffer_1401.value[ i ].xyz.z ) ), 0.0 ), 0.3333333333333333 );
		nodeVar16 = ( f32( i ) * 0.37 );
		nodeVar17 = ( 1.0 / ( pow( length( ( nodeVar0 - ( vec2<f32>( 0.5 ) + ( vec2<f32>( sin( ( ( nodeVar6 * ( 0.6 + ( fract( ( f32( i ) / 3.0 ) ) * 0.9 ) ) ) + nodeVar16 ) ), cos( ( ( nodeVar6 * ( 0.8 + fract( ( ( f32( i ) + 1.0 ) / 4.0 ) ) ) ) + ( nodeVar16 * 1.5 ) ) ) ) * vec2<f32>( 0.5 ) ) ) ) ), object.nodeUniform5 ) + 0.01 ) );
		nodeVar11 = ( nodeVar11 + ( vec3<f32>( ( ( ( 0.2104542553 * nodeVar13 ) + ( 0.793617785 * nodeVar14 ) ) - ( 0.0040720468 * nodeVar15 ) ), ( ( ( 1.9779984951 * nodeVar13 ) - ( 2.428592205 * nodeVar14 ) ) + ( 0.4505937099 * nodeVar15 ) ), ( ( ( 0.0259040371 * nodeVar13 ) + ( 0.7827717662 * nodeVar14 ) ) - ( 0.808675766 * nodeVar15 ) ) ) * vec3<f32>( nodeVar17 ) ) );
		nodeVar12 = ( nodeVar12 + nodeVar17 );

	}

	nodeVar18 = ( nodeVar11 / vec3<f32>( max( 0.0001, nodeVar12 ) ) );
	nodeVar19 = ( ( nodeVar18.x + ( 0.3963377774 * nodeVar18.y ) ) + ( 0.2158037573 * nodeVar18.z ) );
	nodeVar20 = ( ( nodeVar18.x - ( 0.1055613458 * nodeVar18.y ) ) - ( 0.0638541728 * nodeVar18.z ) );
	nodeVar21 = ( ( nodeVar18.x - ( 0.0894841775 * nodeVar18.y ) ) - ( 1.291485548 * nodeVar18.z ) );
	nodeVar22 = clamp( vec3<f32>( ( ( ( 4.0767416621 * ( ( nodeVar19 * nodeVar19 ) * nodeVar19 ) ) - ( 3.3077115913 * ( ( nodeVar20 * nodeVar20 ) * nodeVar20 ) ) ) + ( 0.2309699292 * ( ( nodeVar21 * nodeVar21 ) * nodeVar21 ) ) ), ( ( ( -1.2684380046 * ( ( nodeVar19 * nodeVar19 ) * nodeVar19 ) ) + ( 2.6097574011 * ( ( nodeVar20 * nodeVar20 ) * nodeVar20 ) ) ) - ( 0.3413193965 * ( ( nodeVar21 * nodeVar21 ) * nodeVar21 ) ) ), ( ( ( -0.0041960863 * ( ( nodeVar19 * nodeVar19 ) * nodeVar19 ) ) - ( 0.7034186147 * ( ( nodeVar20 * nodeVar20 ) * nodeVar20 ) ) ) + ( 1.707614701 * ( ( nodeVar21 * nodeVar21 ) * nodeVar21 ) ) ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) );
	nodeVar23 = fragCoord.xy;
	nodeVar24 = ( nodeVar11 / vec3<f32>( max( 0.0001, nodeVar12 ) ) );
	nodeVar25 = ( ( nodeVar24.x + ( 0.3963377774 * nodeVar24.y ) ) + ( 0.2158037573 * nodeVar24.z ) );
	nodeVar26 = ( ( nodeVar24.x - ( 0.1055613458 * nodeVar24.y ) ) - ( 0.0638541728 * nodeVar24.z ) );
	nodeVar27 = ( ( nodeVar24.x - ( 0.0894841775 * nodeVar24.y ) ) - ( 1.291485548 * nodeVar24.z ) );
	nodeVar28 = ( nodeVar23 / vec2<f32>( ( object.nodeUniform6 * 2.1 ) ) );
	nodeVar29 = floor( nodeVar28 );
	nodeVar30 = ( nodeVar29 + vec2<f32>( ( object.nodeUniform7 * 3.3 ) ) );
	nodeVar31 = fract( ( vec3<f32>( nodeVar30.x, nodeVar30.y, nodeVar30.x ) * vec3<f32>( 0.1031 ) ) );
	nodeVar31 = ( nodeVar31 + vec3<f32>( dot( nodeVar31, ( nodeVar31.yzx + vec3<f32>( 33.33 ) ) ) ) );
	nodeVar32 = ( ( nodeVar29 + vec2<f32>( ( object.nodeUniform7 * 1.7 ) ) ) + vec2<f32>( 71.1 ) );
	nodeVar33 = fract( ( vec3<f32>( nodeVar32.x, nodeVar32.y, nodeVar32.x ) * vec3<f32>( 0.1031 ) ) );
	nodeVar33 = ( nodeVar33 + vec3<f32>( dot( nodeVar33, ( nodeVar33.yzx + vec3<f32>( 33.33 ) ) ) ) );
	nodeVar34 = ( ( fract( ( ( nodeVar33.x + nodeVar33.y ) * nodeVar33.z ) ) * 0.3 ) + 0.15 );
	nodeVar35 = ( nodeVar29 + vec2<f32>( 5.5 ) );
	nodeVar36 = fract( ( vec3<f32>( nodeVar35.x, nodeVar35.y, nodeVar35.x ) * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
	nodeVar36 = ( nodeVar36 + vec3<f32>( dot( nodeVar36, ( nodeVar36.yzx + vec3<f32>( 33.33 ) ) ) ) );
	nodeVar22 = mix( nodeVar22, clamp( ( vec3<f32>( ( ( ( 4.0767416621 * ( ( nodeVar25 * nodeVar25 ) * nodeVar25 ) ) - ( 3.3077115913 * ( ( nodeVar26 * nodeVar26 ) * nodeVar26 ) ) ) + ( 0.2309699292 * ( ( nodeVar27 * nodeVar27 ) * nodeVar27 ) ) ), ( ( ( -1.2684380046 * ( ( nodeVar25 * nodeVar25 ) * nodeVar25 ) ) + ( 2.6097574011 * ( ( nodeVar26 * nodeVar26 ) * nodeVar26 ) ) ) - ( 0.3413193965 * ( ( nodeVar27 * nodeVar27 ) * nodeVar27 ) ) ), ( ( ( -0.0041960863 * ( ( nodeVar25 * nodeVar25 ) * nodeVar25 ) ) - ( 0.7034186147 * ( ( nodeVar26 * nodeVar26 ) * nodeVar26 ) ) ) + ( 1.707614701 * ( ( nodeVar27 * nodeVar27 ) * nodeVar27 ) ) ) ) * vec3<f32>( 0.82 ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ), ( step( fract( ( ( nodeVar31.x + nodeVar31.y ) * nodeVar31.z ) ), ( object.nodeUniform8 * smoothstep( -0.1, 0.55, ( ( ( mx_perlin_noise_float_0( ( ( nodeVar23 * vec2<f32>( 0.045 ) ) + vec2<f32>( ( object.nodeUniform7 + 12.7 ) ) ) ) * 1.0 ) + 0.0 ) + ( ( ( mx_perlin_noise_float_0( ( ( nodeVar23 * vec2<f32>( 0.14 ) ) + vec2<f32>( 3.3 ) ) ) * 1.0 ) + 0.0 ) * 0.35 ) ) ) ) ) * ( 1.0 - smoothstep( ( nodeVar34 - 0.08 ), ( nodeVar34 + 0.04 ), length( ( fract( nodeVar28 ) - ( ( fract( ( ( nodeVar36.xx + nodeVar36.yz ) * nodeVar36.zy ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.25 ) ) ) ) ) ) ) );
	nodeVar37 = ( floor( nodeVar23 ) + vec2<f32>( 3.7 ) );
	nodeVar38 = fract( ( vec3<f32>( nodeVar37.x, nodeVar37.y, nodeVar37.x ) * vec3<f32>( 0.1031 ) ) );
	nodeVar38 = ( nodeVar38 + vec3<f32>( dot( nodeVar38, ( nodeVar38.yzx + vec3<f32>( 33.33 ) ) ) ) );
	nodeVar22 = mix( nodeVar22, ( floor( ( ( nodeVar22 * vec3<f32>( object.nodeUniform9 ) ) + vec3<f32>( clamp( ( ( ( ( ( mx_perlin_noise_float_0( ( ( ( nodeVar23 / vec2<f32>( ( object.nodeUniform6 * 1.3 ) ) ) * vec2<f32>( 0.85 ) ) + vec2<f32>( ( object.nodeUniform7 * 9.1 ) ) ) ) * 1.0 ) + 0.0 ) * 0.6 ) + ( fract( ( ( nodeVar38.x + nodeVar38.y ) * nodeVar38.z ) ) * 0.4 ) ) + 0.2 ), 0.0, 1.0 ) ) ) ) / vec3<f32>( object.nodeUniform9 ) ), clamp( object.nodeUniform10, 0.0, 1.0 ) );
	DiffuseColor = vec4<f32>( nodeVar22, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform11 );
	DiffuseColor.w = 1.0;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar39 = ( irradiance + render.nodeUniform12 );
	irradiance = nodeVar39;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;
	nodeVar40 = ( vec4<f32>( indirectDiffuse, 1.0 ) + vec4<f32>( 1.0, 1.0, 1.0, 0.0 ) );
	indirectDiffuse = nodeVar40.xyz;
	ambientOcclusion = 1.0;
	nodeVar41 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar41;
	nodeVar42 = ( indirectDiffuse * DiffuseColor.xyz );
	indirectDiffuse = nodeVar42;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar43 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar43;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar44 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar44;
	nodeVar45 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar45;
	nodeVar46 = max( vec4<f32>( outgoingLight, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar46;

	// result

	output.color = nodeVar46;

	return output;

}
