// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform8_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform31_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform31 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform41_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform41 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform59_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform59 : texture_2d<f32>;

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

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> nodeVar0 : vec3<f32>;
var<private> normalWorldGeometry : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> NORMAL_nodeVar17 : vec4<f32>;
var<private> NORMAL_nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : f32;
var<private> normalView : vec3<f32>;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : vec4<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> nodeVar28 : f32;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : vec4<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : f32;
var<private> nodeVar73 : vec2<f32>;
var<private> nodeVar74 : vec4<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : f32;
var<private> nodeVar77 : f32;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : f32;
var<private> nodeVar81 : vec2<f32>;
var<private> nodeVar82 : vec4<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar85 : f32;
var<private> nodeVar86 : f32;
var<private> nodeVar87 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar88 : f32;
var<private> nodeVar89 : f32;
var<private> nodeVar90 : f32;
var<private> nodeVar91 : vec2<f32>;
var<private> nodeVar92 : vec4<f32>;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : f32;
var<private> nodeVar95 : f32;
var<private> nodeVar96 : f32;
var<private> nodeVar97 : f32;
var<private> nodeVar98 : f32;
var<private> nodeVar99 : vec2<f32>;
var<private> nodeVar100 : vec4<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : f32;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : f32;
var<private> nodeVar115 : f32;
var<private> nodeVar116 : f32;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : vec3<f32>;
var<private> nodeVar120 : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : vec3<f32>;
var<private> nodeVar129 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar130 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar131 : vec3<f32>;
var<private> nodeVar132 : f32;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : vec3<f32>;
var<private> nodeVar138 : vec3<f32>;
var<private> nodeVar139 : vec3<f32>;
var<private> nodeVar140 : f32;
var<private> nodeVar141 : f32;
var<private> nodeVar142 : f32;
var<private> nodeVar143 : vec3<f32>;
var<private> nodeVar144 : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> nodeVar150 : f32;
var<private> nodeVar151 : vec3<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : vec3<f32>;
var<private> nodeVar156 : vec3<f32>;
var<private> nodeVar157 : vec3<f32>;
var<private> nodeVar158 : f32;
var<private> nodeVar159 : f32;
var<private> nodeVar160 : f32;
var<private> nodeVar161 : vec3<f32>;
var<private> nodeVar162 : vec3<f32>;
var<private> nodeVar163 : vec3<f32>;
var<private> nodeVar164 : vec3<f32>;
var<private> nodeVar165 : vec3<f32>;
var<private> nodeVar166 : vec3<f32>;
var<private> nodeVar167 : vec3<f32>;
var<private> nodeVar168 : vec3<f32>;
var<private> nodeVar169 : vec3<f32>;
var<private> nodeVar170 : vec3<f32>;
var<private> nodeVar171 : vec3<f32>;
var<private> nodeVar172 : vec3<f32>;
var<private> nodeVar173 : vec3<f32>;
var<private> nodeVar174 : vec3<f32>;
var<private> nodeVar175 : vec3<f32>;
var<private> nodeVar176 : vec3<f32>;
var<private> nodeVar177 : vec3<f32>;
var<private> nodeVar178 : vec3<f32>;
var<private> nodeVar179 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar180 : vec3<f32>;
var<private> nodeVar181 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar182 : vec3<f32>;
var<private> nodeVar183 : f32;
var<private> nodeVar184 : f32;
var<private> nodeVar185 : f32;
var<private> nodeVar186 : f32;
var<private> nodeVar187 : f32;
var<private> nodeVar188 : f32;
var<private> nodeVar189 : f32;
var<private> nodeVar190 : f32;
var<private> nodeVar191 : f32;
var<private> nodeVar192 : f32;
var<private> nodeVar193 : f32;
var<private> nodeVar194 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar195 : vec3<f32>;
var<private> nodeVar196 : f32;
var<private> nodeVar197 : f32;
var<private> nodeVar198 : vec3<f32>;
var<private> nodeVar199 : vec3<f32>;
var<private> nodeVar200 : f32;
var<private> nodeVar201 : vec3<f32>;
var<private> nodeVar202 : vec3<f32>;
var<private> nodeVar203 : mat3x3<f32>;
var<private> nodeVar204 : vec3<f32>;
var<private> nodeVar205 : vec3<f32>;
var<private> nodeVar206 : vec3<f32>;
var<private> nodeVar207 : vec3<f32>;
var<private> nodeVar208 : f32;
var<private> nodeVar209 : f32;
var<private> nodeVar210 : f32;
var<private> nodeVar211 : vec2<f32>;
var<private> nodeVar212 : vec4<f32>;
var<private> nodeVar213 : vec3<f32>;
var<private> nodeVar214 : vec2<f32>;
var<private> nodeVar215 : f32;
var<private> nodeVar216 : f32;
var<private> nodeVar217 : f32;
var<private> nodeVar218 : mat4x4<f32>;
var<private> nodeVar219 : vec3<f32>;
var<private> nodeVar220 : vec3<f32>;
var<private> nodeVar221 : vec4<f32>;
var<private> nodeVar222 : f32;
var<private> nodeVar223 : vec2<f32>;
var<private> nodeVar224 : f32;
var<private> nodeVar225 : vec3<f32>;
var<private> nodeVar226 : vec3<f32>;
var<private> nodeVar227 : vec4<f32>;
var<private> nodeVar228 : f32;
var<private> nodeVar229 : vec2<f32>;
var<private> nodeVar230 : vec3<f32>;
var<private> nodeVar231 : vec3<f32>;
var<private> nodeVar232 : vec4<f32>;
var<private> nodeVar233 : f32;
var<private> nodeVar234 : vec2<f32>;
var<private> nodeVar235 : vec2<f32>;
var<private> nodeVar236 : vec2<f32>;
var<private> nodeVar237 : f32;
var<private> nodeVar238 : f32;
var<private> nodeVar239 : vec2<f32>;
var<private> nodeVar240 : vec2<f32>;
var<private> nodeVar241 : f32;
var<private> nodeVar242 : f32;
var<private> nodeVar243 : f32;
var<private> nodeVar244 : vec4<f32>;
var<private> nodeVar245 : vec4<f32>;
var<private> nodeVar246 : f32;
var<private> nodeVar247 : vec2<f32>;
var<private> nodeVar248 : vec2<f32>;
var<private> nodeVar249 : vec4<f32>;
var<private> nodeVar250 : vec4<f32>;
var<private> nodeVar251 : vec4<f32>;
var<private> nodeVar252 : vec4<f32>;
var<private> nodeVar253 : f32;
var<private> nodeVar254 : vec2<f32>;
var<private> nodeVar255 : vec2<f32>;
var<private> nodeVar256 : vec4<f32>;
var<private> nodeVar257 : vec4<f32>;
var<private> nodeVar258 : vec4<f32>;
var<private> nodeVar259 : vec4<f32>;
var<private> nodeVar260 : vec3<f32>;
var<private> nodeVar261 : f32;
var<private> nodeVar262 : vec3<f32>;
var<private> nodeVar263 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar264 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar265 : vec3<f32>;
var<private> nodeVar266 : vec4<f32>;

// codes
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


fn mx_hash_int_2 ( x : i32, y : i32, z : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : u32;
	var nodeVar6 : u32;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = 3u;
	nodeVar4 = 0u;
	nodeVar5 = 0u;
	nodeVar6 = 0u;
	nodeVar6 = ( ( 3735928559u + ( nodeVar3 << 2u ) ) + 13u );
	nodeVar5 = nodeVar6;
	nodeVar4 = nodeVar5;
	nodeVar4 = ( nodeVar4 + u32( nodeVar2 ) );
	nodeVar5 = ( nodeVar5 + u32( nodeVar1 ) );
	nodeVar6 = ( nodeVar6 + u32( nodeVar0 ) );

	return mx_bjfinal( nodeVar4, nodeVar5, nodeVar6 );

}


fn mx_hash_vec3_1 ( x : i32, y : i32, z : i32 ) -> vec3<u32> {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : u32;
	var nodeVar4 : vec3<u32>;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = mx_hash_int_2( nodeVar2, nodeVar1, nodeVar0 );
	nodeVar4 = vec3<u32>( 0u, 0u, 0u );
	nodeVar4.x = ( nodeVar3 & 255u );
	nodeVar4.y = ( ( nodeVar3 >> 8u ) & 255u );
	nodeVar4.z = ( ( nodeVar3 >> 16u ) & 255u );

	return nodeVar4;

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


fn mx_gradient_float_1 ( hash : u32, x : f32, y : f32, z : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = hash;
	nodeVar4 = ( nodeVar3 & 15u );
	nodeVar5 = mx_select( ( nodeVar4 < 8u ), nodeVar2, nodeVar1 );
	nodeVar6 = mx_select( ( nodeVar4 < 4u ), nodeVar1, mx_select( ( ( nodeVar4 == 12u ) || ( nodeVar4 == 14u ) ), nodeVar2, nodeVar0 ) );

	return ( mx_negate_if( nodeVar5, bool( ( nodeVar4 & 1u ) ) ) + mx_negate_if( nodeVar6, bool( ( nodeVar4 & 2u ) ) ) );

}


fn mx_gradient_vec3_1 ( hash : vec3<u32>, x : f32, y : f32, z : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : vec3<u32>;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = hash;

	return vec3<f32>( mx_gradient_float_1( nodeVar3.x, nodeVar2, nodeVar1, nodeVar0 ), mx_gradient_float_1( nodeVar3.y, nodeVar2, nodeVar1, nodeVar0 ), mx_gradient_float_1( nodeVar3.z, nodeVar2, nodeVar1, nodeVar0 ) );

}


fn mx_trilerp_1 ( v0 : vec3<f32>, v1 : vec3<f32>, v2 : vec3<f32>, v3 : vec3<f32>, v4 : vec3<f32>, v5 : vec3<f32>, v6 : vec3<f32>, v7 : vec3<f32>, s : f32, t : f32, r : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : vec3<f32>;
	var nodeVar6 : vec3<f32>;
	var nodeVar7 : vec3<f32>;
	var nodeVar8 : vec3<f32>;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : vec3<f32>;
	var nodeVar11 : f32;
	var nodeVar12 : f32;
	var nodeVar13 : f32;

	nodeVar0 = r;
	nodeVar1 = t;
	nodeVar2 = s;
	nodeVar3 = v7;
	nodeVar4 = v6;
	nodeVar5 = v5;
	nodeVar6 = v4;
	nodeVar7 = v3;
	nodeVar8 = v2;
	nodeVar9 = v1;
	nodeVar10 = v0;
	nodeVar11 = ( 1.0 - nodeVar2 );
	nodeVar12 = ( 1.0 - nodeVar1 );
	nodeVar13 = ( 1.0 - nodeVar0 );

	return ( ( vec3<f32>( nodeVar13 ) * ( ( vec3<f32>( nodeVar12 ) * ( ( nodeVar10 * vec3<f32>( nodeVar11 ) ) + ( nodeVar9 * vec3<f32>( nodeVar2 ) ) ) ) + ( vec3<f32>( nodeVar1 ) * ( ( nodeVar8 * vec3<f32>( nodeVar11 ) ) + ( nodeVar7 * vec3<f32>( nodeVar2 ) ) ) ) ) ) + ( vec3<f32>( nodeVar0 ) * ( ( vec3<f32>( nodeVar12 ) * ( ( nodeVar6 * vec3<f32>( nodeVar11 ) ) + ( nodeVar5 * vec3<f32>( nodeVar2 ) ) ) ) + ( vec3<f32>( nodeVar1 ) * ( ( nodeVar4 * vec3<f32>( nodeVar11 ) ) + ( nodeVar3 * vec3<f32>( nodeVar2 ) ) ) ) ) ) );

}


fn mx_gradient_scale3d_1 ( v : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;

	nodeVar0 = v;

	return ( vec3<f32>( 0.982 ) * nodeVar0 );

}


fn mx_perlin_noise_vec3_1 ( p : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;
	var nodeVar10 : f32;
	var nodeVar11 : f32;
	var nodeVar12 : f32;
	var nodeVar13 : vec3<f32>;

	nodeVar0 = p;
	nodeVar1 = 0;
	nodeVar2 = 0;
	nodeVar3 = 0;
	nodeVar4 = nodeVar0.x;
	nodeVar1 = mx_floor( nodeVar4 );
	nodeVar5 = ( nodeVar4 - f32( nodeVar1 ) );
	nodeVar6 = nodeVar0.y;
	nodeVar2 = mx_floor( nodeVar6 );
	nodeVar7 = ( nodeVar6 - f32( nodeVar2 ) );
	nodeVar8 = nodeVar0.z;
	nodeVar3 = mx_floor( nodeVar8 );
	nodeVar9 = ( nodeVar8 - f32( nodeVar3 ) );
	nodeVar10 = mx_fade( nodeVar5 );
	nodeVar11 = mx_fade( nodeVar7 );
	nodeVar12 = mx_fade( nodeVar9 );
	nodeVar13 = mx_trilerp_1( mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, nodeVar2, nodeVar3 ), nodeVar5, nodeVar7, nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), nodeVar2, nodeVar3 ), ( nodeVar5 - 1.0 ), nodeVar7, nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, ( nodeVar2 + 1 ), nodeVar3 ), nodeVar5, ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), nodeVar3 ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, nodeVar2, ( nodeVar3 + 1 ) ), nodeVar5, nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), nodeVar2, ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), nodeVar5, ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), nodeVar10, nodeVar11, nodeVar12 );

	return mx_gradient_scale3d_1( nodeVar13 );

}


fn V_GGX_SmithCorrelated ( alpha : f32, dotNL : f32, dotNV : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = ( alpha * alpha );

	return ( 0.5 / max( ( ( dotNL * sqrt( ( nodeVar0 + ( ( 1.0 - nodeVar0 ) * ( dotNV * dotNV ) ) ) ) ) + ( dotNV * sqrt( ( nodeVar0 + ( ( 1.0 - nodeVar0 ) * ( dotNL * dotNL ) ) ) ) ) ), 0.000001 ) );

}


fn D_GGX ( alpha : f32, dotNH : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;

	nodeVar0 = ( alpha * alpha );
	nodeVar1 = ( 1.0 - ( ( dotNH * dotNH ) * ( 1.0 - nodeVar0 ) ) );

	return ( ( nodeVar0 / ( nodeVar1 * nodeVar1 ) ) * 0.3183098861837907 );

}


fn roughnessToMip ( roughness : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = 0.0;

	if ( ( roughness >= 0.8 ) ) {

		nodeVar0 = ( ( ( ( 1.0 - roughness ) * ( -1.0 - -2.0 ) ) / ( 1.0 - 0.8 ) ) + -2.0 );
		

	} else {


		if ( ( roughness >= 0.4 ) ) {

			nodeVar0 = ( ( ( ( 0.8 - roughness ) * ( 2.0 - -1.0 ) ) / ( 0.8 - 0.4 ) ) + -1.0 );
			

		} else {


			if ( ( roughness >= 0.305 ) ) {

				nodeVar0 = ( ( ( ( 0.4 - roughness ) * ( 3.0 - 2.0 ) ) / ( 0.4 - 0.305 ) ) + 2.0 );
				

			} else {


				if ( ( roughness >= 0.21 ) ) {

					nodeVar0 = ( ( ( ( 0.305 - roughness ) * ( 4.0 - 3.0 ) ) / ( 0.305 - 0.21 ) ) + 3.0 );
					

				} else {

					nodeVar0 = ( -2.0 * log2( ( 1.16 * roughness ) ) );
					

				}

				

			}

			

		}

		

	}


	return nodeVar0;

}


fn getFace ( direction : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = abs( direction );
	nodeVar1 = -1.0;

	if ( ( nodeVar0.x > nodeVar0.z ) ) {


		if ( ( nodeVar0.x > nodeVar0.y ) ) {


			if ( ( direction.x > 0.0 ) ) {

				nodeVar2 = 0.0;

			} else {

				nodeVar2 = 3.0;

			}

			nodeVar1 = nodeVar2;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar3 = 1.0;

			} else {

				nodeVar3 = 4.0;

			}

			nodeVar1 = nodeVar3;
			

		}

		

	} else {


		if ( ( nodeVar0.z > nodeVar0.y ) ) {


			if ( ( direction.z > 0.0 ) ) {

				nodeVar4 = 2.0;

			} else {

				nodeVar4 = 5.0;

			}

			nodeVar1 = nodeVar4;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar5 = 1.0;

			} else {

				nodeVar5 = 4.0;

			}

			nodeVar1 = nodeVar5;
			

		}

		

	}


	return nodeVar1;

}


fn getUV ( direction : vec3<f32>, face : f32 ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;

	nodeVar0 = vec2<f32>( 0.0, 0.0 );

	if ( ( face == 0.0 ) ) {

		nodeVar0 = ( vec2<f32>( direction.z, direction.y ) / vec2<f32>( abs( direction.x ) ) );
		

	} else {


		if ( ( face == 1.0 ) ) {

			nodeVar0 = ( vec2<f32>( ( - direction.x ), ( - direction.z ) ) / vec2<f32>( abs( direction.y ) ) );
			

		} else {


			if ( ( face == 2.0 ) ) {

				nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.y ) / vec2<f32>( abs( direction.z ) ) );
				

			} else {


				if ( ( face == 3.0 ) ) {

					nodeVar0 = ( vec2<f32>( ( - direction.z ), direction.y ) / vec2<f32>( abs( direction.x ) ) );
					

				} else {


					if ( ( face == 4.0 ) ) {

						nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.z ) / vec2<f32>( abs( direction.y ) ) );
						

					} else {

						nodeVar0 = ( vec2<f32>( direction.x, direction.y ) / vec2<f32>( abs( direction.z ) ) );
						

					}

					

				}

				

			}

			

		}

		

	}


	return ( vec2<f32>( 0.5 ) * ( nodeVar0 + vec2<f32>( 1.0 ) ) );

}


fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_normalWorldGeometry : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	Metalness = object.nodeUniform2;
	nodeVar0 = ( v_positionWorld * vec3<f32>( object.nodeUniform5 ) );
	normalWorldGeometry = normalize( v_normalWorldGeometry );
	nodeVar2 = normalWorldGeometry;

	if ( ( nodeVar2.x < 0.0 ) ) {

		nodeVar3 = -1.0;

	} else {

		nodeVar3 = 1.0;

	}

	nodeVar4 = nodeVar3;

	if ( ( nodeVar2.y < 0.0 ) ) {

		nodeVar5 = -1.0;

	} else {

		nodeVar5 = 1.0;

	}

	nodeVar6 = nodeVar5;

	if ( ( nodeVar2.z < 0.0 ) ) {

		nodeVar7 = -1.0;

	} else {

		nodeVar7 = 1.0;

	}

	nodeVar8 = nodeVar7;
	nodeVar9 = textureSample( nodeUniform8, nodeUniform8_sampler, vec2<f32>( ( nodeVar0.z * nodeVar4 ), nodeVar0.y ) );
	nodeVar10 = ( ( nodeVar9.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) );
	nodeVar11 = textureSample( nodeUniform8, nodeUniform8_sampler, vec2<f32>( ( nodeVar0.x * nodeVar6 ), nodeVar0.z ) );
	nodeVar12 = ( ( nodeVar11.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) );
	nodeVar13 = textureSample( nodeUniform8, nodeUniform8_sampler, vec2<f32>( ( nodeVar0.x * ( - nodeVar8 ) ), nodeVar0.y ) );
	nodeVar14 = ( ( nodeVar13.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) );
	nodeVar10.z = ( nodeVar10.z * nodeVar4 );
	nodeVar12.z = ( nodeVar12.z * nodeVar6 );
	nodeVar14.z = ( nodeVar14.z * ( - nodeVar8 ) );
	nodeVar15 = pow( abs( nodeVar2 ), vec3<f32>( object.nodeUniform9 ) );
	nodeVar15 = ( nodeVar15 / vec3<f32>( max( ( ( nodeVar15.x + nodeVar15.y ) + nodeVar15.z ), 0.0001 ) ) );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	nodeVar16 = normalize( ( render.cameraViewMatrix * vec4<f32>( normalize( ( ( ( vec3<f32>( ( nodeVar10.xy + nodeVar2.zy ), ( abs( nodeVar10.z ) * nodeVar2.x ) ).zyx * vec3<f32>( nodeVar15.x ) ) + ( vec3<f32>( ( nodeVar12.xy + nodeVar2.xz ), ( abs( nodeVar12.z ) * nodeVar2.y ) ).xzy * vec3<f32>( nodeVar15.y ) ) ) + ( vec3<f32>( ( nodeVar14.xy + nodeVar2.xy ), ( abs( nodeVar14.z ) * nodeVar2.z ) ) * vec3<f32>( nodeVar15.z ) ) ) ), 1.0 ) ) );
	NORMAL_nodeVar17 = normalize( mix( vec4<f32>( NORMAL_normalView, 1.0 ), nodeVar16, object.nodeUniform11 ) );
	NORMAL_nodeVar18 = NORMAL_nodeVar17.xyz;
	nodeVar19 = ( v_positionWorld * vec3<f32>( object.nodeUniform12 ) );
	nodeVar20 = fwidth( nodeVar19 );
	nodeVar21 = ( 1.0 - smoothstep( 0.25, 0.75, max( max( nodeVar20.x, nodeVar20.y ), nodeVar20.z ) ) );

	if ( ( object.nodeUniform13 > 0.00001 ) ) {

		NORMAL_nodeVar18 = normalize( ( NORMAL_nodeVar18 + ( ( ( render.cameraViewMatrix * vec4<f32>( ( ( mx_perlin_noise_vec3_1( ( nodeVar19 + ( vec3<f32>( 1.0, -1.0, -1.0 ) * vec3<f32>( ( object.nodeUniform14 * object.nodeUniform15 ) ) ) ) ) * vec3<f32>( 1.0 ) ) + vec3<f32>( 0.0 ) ), 0.0 ) ).xyz * vec3<f32>( object.nodeUniform13 ) ) * vec3<f32>( nodeVar21 ) ) ) );
		

	}

	normalView = NORMAL_nodeVar18;
	nodeVar22 = normalize( mix( vec4<f32>( normalView, 1.0 ), nodeVar16, object.nodeUniform11 ) );
	nodeVar23 = dpdx( nodeVar22 );
	nodeVar24 = - dpdy( nodeVar22 );
	nodeVar25 = clamp( sqrt( ( ( object.nodeUniform3 * object.nodeUniform3 ) + ( min( 0.18, ( ( dot( nodeVar23, nodeVar23 ) + dot( nodeVar24, nodeVar24 ) ) * 0.5 ) ) * object.nodeUniform16 ) ) ), 0.0, 1.0 );
	nodeVar26 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( nodeVar25, 0.0525 ) + max( max( nodeVar26.x, nodeVar26.y ), nodeVar26.z ) ), 1.0 );
	nodeVar27 = nodeVar22.xyz;
	IOR = object.nodeUniform17;
	nodeVar28 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	SpecularColor = ( min( ( vec3<f32>( ( nodeVar28 * nodeVar28 ) ) * object.nodeUniform18 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform19 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform19, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform2 ) ) );
	positionViewDirection = vec3<f32>( 0.0, 0.0, 1.0 );
	EmissiveColor = ( max( ( ( ( object.nodeUniform20 * vec3<f32>( pow( ( 1.0 - abs( dot( normalView, positionViewDirection ) ) ), object.nodeUniform21 ) ) ) * vec3<f32>( object.nodeUniform22 ) ) * vec3<f32>( ( ( sin( ( render.nodeUniform23 * 2.0 ) ) * object.nodeUniform24 ) + 1.0 ) ) ), vec3<f32>( 0.0 ) ) + ( ( object.nodeUniform25 * vec3<f32>( smoothstep( ( object.nodeUniform26 - object.nodeUniform27 ), ( object.nodeUniform26 + object.nodeUniform27 ), pow( max( dot( normalView, normalize( ( normalize( ( render.nodeUniform28 - v_positionView ) ) + positionViewDirection ) ) ), 0.0 ), object.nodeUniform29 ) ) ) ) * vec3<f32>( object.nodeUniform30 ) ) );
	nodeVar29 = dot( normalView, positionViewDirection );
	nodeVar30 = textureSample( nodeUniform31, nodeUniform31_sampler, vec2<f32>( Roughness, clamp( nodeVar29, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar30;
	nodeVar31 = ( dfg.x + dfg.y );
	nodeVar32 = ( 1.0 / nodeVar31 );
	nodeVar33 = nodeVar32;
	nodeVar34 = ( nodeVar33 - 1.0 );
	nodeVar35 = ( SpecularColorBlended * vec3<f32>( nodeVar34 ) );
	nodeVar36 = ( nodeVar35 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar36;
	nodeVar37 = ( render.nodeUniform32 - render.nodeUniform33 );
	nodeVar38 = vec4<f32>( nodeVar37, 0.0 );
	nodeVar39 = ( render.cameraViewMatrix * nodeVar38 );
	nodeVar40 = normalize( nodeVar39.xyz );
	nodeVar41 = nodeVar40;
	nodeVar42 = dot( normalView, nodeVar41 );
	nodeVar43 = ( vec3<f32>( clamp( nodeVar42, 0.0, 1.0 ) ) * render.nodeUniform34 );
	nodeVar44 = nodeVar43;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar45 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar46 = ( nodeVar44 * nodeVar45 );
	nodeVar47 = ( nodeVar41 + positionViewDirection );
	nodeVar48 = normalize( nodeVar47 );
	nodeVar49 = dot( positionViewDirection, nodeVar48 );
	nodeVar50 = clamp( nodeVar49, 0.0, 1.0 );
	nodeVar51 = exp2( ( ( ( nodeVar50 * -5.55473 ) - 6.98316 ) * nodeVar50 ) );
	nodeVar52 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar51 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar51 ) ) );
	nodeVar53 = ( vec3<f32>( 1.0 ) - nodeVar52 );
	nodeVar54 = nodeVar53;
	nodeVar55 = ( nodeVar46 * nodeVar54 );
	nodeVar56 = ( directDiffuse + nodeVar55 );
	directDiffuse = nodeVar56;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar57 = normalize( ( nodeVar41 + positionViewDirection ) );
	nodeVar58 = clamp( dot( positionViewDirection, nodeVar57 ), 0.0, 1.0 );
	nodeVar59 = exp2( ( ( ( nodeVar58 * -5.55473 ) - 6.98316 ) * nodeVar58 ) );
	nodeVar60 = ( Roughness * Roughness );
	nodeVar61 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar59 ) ) ) + vec3<f32>( ( 1.0 * nodeVar59 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar60, clamp( dot( normalView, nodeVar41 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar60, clamp( dot( normalView, nodeVar57 ), 0.0, 1.0 ) ) ) );
	nodeVar62 = ( nodeVar44 * nodeVar61 );
	nodeVar63 = ( nodeVar62 * multiScatteringCompensation );
	nodeVar64 = ( directSpecular + nodeVar63 );
	directSpecular = nodeVar64;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar65 = ( irradiance + render.nodeUniform35 );
	irradiance = nodeVar65;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar66 = clamp( roughnessToMip( Roughness ), -2.0, object.nodeUniform36 );
	nodeVar67 = floor( nodeVar66 );
	nodeVar68 = nodeVar67;
	nodeVar69 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	nodeVar70 = getFace( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( nodeVar69.x, ( - nodeVar69.y ), nodeVar69.z ), 1.0 ) ).xyz );
	nodeVar71 = max( ( 4.0 - nodeVar68 ), 0.0 );
	nodeVar68 = max( nodeVar68, 4.0 );
	nodeVar72 = exp2( nodeVar68 );
	nodeVar73 = ( ( getUV( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( nodeVar69.x, ( - nodeVar69.y ), nodeVar69.z ), 1.0 ) ).xyz, nodeVar70 ) * vec2<f32>( ( nodeVar72 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar70 > 2.0 ) ) {

		nodeVar73.y = ( nodeVar73.y + nodeVar72 );
		nodeVar70 = ( nodeVar70 - 3.0 );
		

	}

	nodeVar73.x = ( nodeVar73.x + ( nodeVar70 * nodeVar72 ) );
	nodeVar73.x = ( nodeVar73.x + ( nodeVar71 * ( 3.0 * 16.0 ) ) );
	nodeVar73.y = ( nodeVar73.y + ( 4.0 * ( exp2( object.nodeUniform36 ) - nodeVar72 ) ) );
	nodeVar73.x = ( nodeVar73.x * object.nodeUniform39 );
	nodeVar73.y = ( nodeVar73.y * object.nodeUniform40 );
	nodeVar74 = textureSampleGrad( nodeUniform41, nodeUniform41_sampler, nodeVar73, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar75 = nodeVar74.xyz;
	nodeVar76 = fract( nodeVar66 );

	if ( ( nodeVar76 != 0.0 ) ) {

		nodeVar77 = ( nodeVar67 + 1.0 );
		nodeVar78 = getFace( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( nodeVar69.x, ( - nodeVar69.y ), nodeVar69.z ), 1.0 ) ).xyz );
		nodeVar79 = max( ( 4.0 - nodeVar77 ), 0.0 );
		nodeVar77 = max( nodeVar77, 4.0 );
		nodeVar80 = exp2( nodeVar77 );
		nodeVar81 = ( ( getUV( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( nodeVar69.x, ( - nodeVar69.y ), nodeVar69.z ), 1.0 ) ).xyz, nodeVar78 ) * vec2<f32>( ( nodeVar80 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar78 > 2.0 ) ) {

			nodeVar81.y = ( nodeVar81.y + nodeVar80 );
			nodeVar78 = ( nodeVar78 - 3.0 );
			

		}

		nodeVar81.x = ( nodeVar81.x + ( nodeVar78 * nodeVar80 ) );
		nodeVar81.x = ( nodeVar81.x + ( nodeVar79 * ( 3.0 * 16.0 ) ) );
		nodeVar81.y = ( nodeVar81.y + ( 4.0 * ( exp2( object.nodeUniform36 ) - nodeVar80 ) ) );
		nodeVar81.x = ( nodeVar81.x * object.nodeUniform39 );
		nodeVar81.y = ( nodeVar81.y * object.nodeUniform40 );
		nodeVar82 = textureSampleGrad( nodeUniform41, nodeUniform41_sampler, nodeVar81, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar83 = nodeVar82.xyz;
		nodeVar75 = mix( nodeVar75, nodeVar83, nodeVar76 );
		

	}

	nodeVar84 = ( radiance + ( nodeVar75 * vec3<f32>( object.nodeUniform42 ) ) );
	radiance = nodeVar84;
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar85 = clamp( roughnessToMip( 1.0 ), -2.0, object.nodeUniform36 );
	nodeVar86 = floor( nodeVar85 );
	nodeVar87 = nodeVar86;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar88 = getFace( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
	nodeVar89 = max( ( 4.0 - nodeVar87 ), 0.0 );
	nodeVar87 = max( nodeVar87, 4.0 );
	nodeVar90 = exp2( nodeVar87 );
	nodeVar91 = ( ( getUV( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar88 ) * vec2<f32>( ( nodeVar90 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar88 > 2.0 ) ) {

		nodeVar91.y = ( nodeVar91.y + nodeVar90 );
		nodeVar88 = ( nodeVar88 - 3.0 );
		

	}

	nodeVar91.x = ( nodeVar91.x + ( nodeVar88 * nodeVar90 ) );
	nodeVar91.x = ( nodeVar91.x + ( nodeVar89 * ( 3.0 * 16.0 ) ) );
	nodeVar91.y = ( nodeVar91.y + ( 4.0 * ( exp2( object.nodeUniform36 ) - nodeVar90 ) ) );
	nodeVar91.x = ( nodeVar91.x * object.nodeUniform39 );
	nodeVar91.y = ( nodeVar91.y * object.nodeUniform40 );
	nodeVar92 = textureSampleGrad( nodeUniform41, nodeUniform41_sampler, nodeVar91, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar93 = nodeVar92.xyz;
	nodeVar94 = fract( nodeVar85 );

	if ( ( nodeVar94 != 0.0 ) ) {

		nodeVar95 = ( nodeVar86 + 1.0 );
		nodeVar96 = getFace( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
		nodeVar97 = max( ( 4.0 - nodeVar95 ), 0.0 );
		nodeVar95 = max( nodeVar95, 4.0 );
		nodeVar98 = exp2( nodeVar95 );
		nodeVar99 = ( ( getUV( ( object.nodeUniform37 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar96 ) * vec2<f32>( ( nodeVar98 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar96 > 2.0 ) ) {

			nodeVar99.y = ( nodeVar99.y + nodeVar98 );
			nodeVar96 = ( nodeVar96 - 3.0 );
			

		}

		nodeVar99.x = ( nodeVar99.x + ( nodeVar96 * nodeVar98 ) );
		nodeVar99.x = ( nodeVar99.x + ( nodeVar97 * ( 3.0 * 16.0 ) ) );
		nodeVar99.y = ( nodeVar99.y + ( 4.0 * ( exp2( object.nodeUniform36 ) - nodeVar98 ) ) );
		nodeVar99.x = ( nodeVar99.x * object.nodeUniform39 );
		nodeVar99.y = ( nodeVar99.y * object.nodeUniform40 );
		nodeVar100 = textureSampleGrad( nodeUniform41, nodeUniform41_sampler, nodeVar99, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar101 = nodeVar100.xyz;
		nodeVar93 = mix( nodeVar93, nodeVar101, nodeVar94 );
		

	}

	nodeVar102 = ( iblIrradiance + ( ( nodeVar93 * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform42 ) ) );
	iblIrradiance = nodeVar102;
	nodeVar103 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar104 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar105 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar106 = ( SpecularF90 * dfg.y );
	nodeVar107 = ( nodeVar105 + vec3<f32>( nodeVar106 ) );
	nodeVar108 = ( nodeVar103 + nodeVar107 );
	nodeVar103 = nodeVar108;
	nodeVar109 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar110 = nodeVar109;
	nodeVar111 = ( nodeVar110 * vec3<f32>( 0.047619 ) );
	nodeVar112 = ( SpecularColor + nodeVar111 );
	nodeVar113 = ( nodeVar107 * nodeVar112 );
	nodeVar114 = ( dfg.x + dfg.y );
	nodeVar115 = ( 1.0 - nodeVar114 );
	nodeVar116 = nodeVar115;
	nodeVar117 = ( vec3<f32>( nodeVar116 ) * nodeVar112 );
	nodeVar118 = ( vec3<f32>( 1.0 ) - nodeVar117 );
	nodeVar119 = nodeVar118;
	nodeVar120 = ( nodeVar113 / nodeVar119 );
	nodeVar121 = ( nodeVar120 * vec3<f32>( nodeVar116 ) );
	nodeVar122 = ( nodeVar104 + nodeVar121 );
	nodeVar104 = nodeVar122;
	nodeVar123 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar124 = ( irradiance * nodeVar123 );
	nodeVar125 = ( nodeVar103 + nodeVar104 );
	nodeVar126 = ( vec3<f32>( 1.0 ) - nodeVar125 );
	nodeVar127 = nodeVar126;
	nodeVar128 = ( nodeVar124 * nodeVar127 );
	nodeVar129 = nodeVar128;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar130 = ( indirectDiffuse + nodeVar129 );
	indirectDiffuse = nodeVar130;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar131 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar132 = ( SpecularF90 * dfg.y );
	nodeVar133 = ( nodeVar131 + vec3<f32>( nodeVar132 ) );
	nodeVar134 = ( singleScatteringDielectric + nodeVar133 );
	singleScatteringDielectric = nodeVar134;
	nodeVar135 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar136 = nodeVar135;
	nodeVar137 = ( nodeVar136 * vec3<f32>( 0.047619 ) );
	nodeVar138 = ( SpecularColor + nodeVar137 );
	nodeVar139 = ( nodeVar133 * nodeVar138 );
	nodeVar140 = ( dfg.x + dfg.y );
	nodeVar141 = ( 1.0 - nodeVar140 );
	nodeVar142 = nodeVar141;
	nodeVar143 = ( vec3<f32>( nodeVar142 ) * nodeVar138 );
	nodeVar144 = ( vec3<f32>( 1.0 ) - nodeVar143 );
	nodeVar145 = nodeVar144;
	nodeVar146 = ( nodeVar139 / nodeVar145 );
	nodeVar147 = ( nodeVar146 * vec3<f32>( nodeVar142 ) );
	nodeVar148 = ( multiScatteringDielectric + nodeVar147 );
	multiScatteringDielectric = nodeVar148;
	nodeVar149 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar150 = ( SpecularF90 * dfg.y );
	nodeVar151 = ( nodeVar149 + vec3<f32>( nodeVar150 ) );
	nodeVar152 = ( singleScatteringMetallic + nodeVar151 );
	singleScatteringMetallic = nodeVar152;
	nodeVar153 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar154 = nodeVar153;
	nodeVar155 = ( nodeVar154 * vec3<f32>( 0.047619 ) );
	nodeVar156 = ( DiffuseColor.xyz + nodeVar155 );
	nodeVar157 = ( nodeVar151 * nodeVar156 );
	nodeVar158 = ( dfg.x + dfg.y );
	nodeVar159 = ( 1.0 - nodeVar158 );
	nodeVar160 = nodeVar159;
	nodeVar161 = ( vec3<f32>( nodeVar160 ) * nodeVar156 );
	nodeVar162 = ( vec3<f32>( 1.0 ) - nodeVar161 );
	nodeVar163 = nodeVar162;
	nodeVar164 = ( nodeVar157 / nodeVar163 );
	nodeVar165 = ( nodeVar164 * vec3<f32>( nodeVar160 ) );
	nodeVar166 = ( multiScatteringMetallic + nodeVar165 );
	multiScatteringMetallic = nodeVar166;
	nodeVar167 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar168 = ( radiance * nodeVar167 );
	nodeVar169 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	nodeVar170 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar171 = ( nodeVar169 * nodeVar170 );
	nodeVar172 = ( nodeVar168 + nodeVar171 );
	nodeVar173 = nodeVar172;
	nodeVar174 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar175 = ( vec3<f32>( 1.0 ) - nodeVar174 );
	nodeVar176 = nodeVar175;
	nodeVar177 = ( DiffuseContribution * nodeVar176 );
	nodeVar178 = ( nodeVar177 * nodeVar170 );
	nodeVar179 = nodeVar178;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar180 = ( indirectSpecular + nodeVar173 );
	indirectSpecular = nodeVar180;
	nodeVar181 = ( indirectDiffuse + nodeVar179 );
	indirectDiffuse = nodeVar181;
	ambientOcclusion = 1.0;
	nodeVar182 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar182;
	nodeVar183 = dot( normalView, positionViewDirection );
	nodeVar184 = ( clamp( nodeVar183, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar185 = ( Roughness * -16.0 );
	nodeVar186 = ( 1.0 - nodeVar185 );
	nodeVar187 = nodeVar186;
	nodeVar188 = ( - nodeVar187 );
	nodeVar189 = exp2( nodeVar188 );
	nodeVar190 = pow( nodeVar184, nodeVar189 );
	nodeVar191 = ( 1.0 - nodeVar190 );
	nodeVar192 = nodeVar191;
	nodeVar193 = ( ambientOcclusion - nodeVar192 );
	nodeVar194 = ( indirectSpecular * vec3<f32>( clamp( nodeVar193, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar194;
	nodeVar195 = ( directDiffuse + indirectDiffuse );
	nodeVar196 = object.nodeUniform43;
	nodeVar197 = object.nodeUniform44;
	nodeVar198 = normalize( ( v_positionWorld - render.cameraPosition ) );
	nodeVar199 = normalWorld;

	if ( ( dot( nodeVar199, nodeVar198 ) > 0.0 ) ) {

		nodeVar200 = -1.0;

	} else {

		nodeVar200 = 1.0;

	}

	nodeVar201 = ( nodeVar199 * vec3<f32>( nodeVar200 ) );
	nodeVar202 = vec3<f32>( length( object.nodeUniform46[ 0u ].xyz ), length( object.nodeUniform46[ 1u ].xyz ), length( object.nodeUniform46[ 2u ].xyz ) );
	nodeVar203 = mat3x3<f32>( ( object.nodeUniform46[ 0u ].xyz / vec3<f32>( nodeVar202.x ) ), ( object.nodeUniform46[ 1u ].xyz / vec3<f32>( nodeVar202.y ) ), ( object.nodeUniform46[ 2u ].xyz / vec3<f32>( nodeVar202.z ) ) );
	nodeVar204 = ( nodeVar202 * vec3<f32>( object.nodeUniform47 ) );
	nodeVar205 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar206 = ( v_positionWorld * vec3<f32>( object.nodeUniform48 ) );
	nodeVar207 = fwidth( nodeVar206 );
	nodeVar208 = ( 1.0 - smoothstep( 0.25, 0.75, max( max( nodeVar207.x, nodeVar207.y ), nodeVar207.z ) ) );

	if ( ( object.nodeUniform49 > 0.00001 ) ) {

		nodeVar205 = ( ( ( nodeVar203 * ( ( mx_perlin_noise_vec3_1( ( nodeVar206 + vec3<f32>( ( object.nodeUniform50 * object.nodeUniform51 ) ) ) ) * vec3<f32>( 1.0 ) ) + vec3<f32>( 0.0 ) ) ) * vec3<f32>( object.nodeUniform49 ) ) * vec3<f32>( nodeVar208 ) );
		

	}

	nodeVar209 = clamp( nodeVar25, 0.0, 1.0 );
	nodeVar210 = ( nodeVar209 * nodeVar209 );
	nodeVar211 = floor( fragCoord.xy );
	nodeVar212 = vec4<f32>( interleavedGradientNoise( nodeVar211 ), interleavedGradientNoise( ( nodeVar211 + vec2<f32>( 47.0, 17.0 ) ) ), interleavedGradientNoise( ( nodeVar211 + vec2<f32>( -23.0, 71.0 ) ) ), interleavedGradientNoise( ( nodeVar211 + vec2<f32>( 89.0, -31.0 ) ) ) );
	nodeVar213 = normalize( ( ( nodeVar201 + ( ( ( ( ( nodeVar212.xyw - vec3<f32>( 0.5 ) ) * vec3<f32>( 2.0 ) ) * vec3<f32>( nodeVar210 ) ) * vec3<f32>( object.nodeUniform52 ) ) * vec3<f32>( 0.35 ) ) ) + nodeVar205 ) );
	nodeVar214 = ( ( fragCoord.xy - render.nodeUniform53.xy ) / render.nodeUniform53.zw );
	nodeVar215 = smoothstep( 0.0, 1.0, ( min( min( nodeVar214.x, ( 1.0 - nodeVar214.x ) ), min( nodeVar214.y, ( 1.0 - nodeVar214.y ) ) ) / max( object.nodeUniform54, 0.00001 ) ) );
	nodeVar216 = max( IOR, 1.0 );
	nodeVar217 = ( max( ( nodeVar216 - 1.0 ), 0.0 ) * ( ( object.nodeUniform55 * 0.025 ) + object.nodeUniform56 ) );
	nodeVar218 = ( render.cameraProjectionMatrix * render.cameraViewMatrix );
	nodeVar219 = normalize( refract( nodeVar198, nodeVar213, ( 1.0 / max( nodeVar216, 1.0 ) ) ) );
	nodeVar220 = ( nodeVar219 * nodeVar204 );
	nodeVar221 = ( nodeVar218 * vec4<f32>( ( ( v_positionWorld + nodeVar220 ) + ( nodeVar219 * vec3<f32>( nodeVar196 ) ) ), 1.0 ) );
	nodeVar222 = max( nodeVar221.w, 0.0001 );
	nodeVar223 = ( ( ( clamp( mix( nodeVar214, vec2<f32>( ( ( ( nodeVar221.x / nodeVar222 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar221.y / nodeVar222 ) * -0.5 ) + 0.5 ) ), nodeVar215 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform53.zw ) + render.nodeUniform53.xy ) / render.nodeUniform58 );
	nodeVar224 = length( nodeVar220 );
	nodeVar225 = normalize( refract( nodeVar198, nodeVar213, ( 1.0 / max( ( nodeVar216 - nodeVar217 ), 1.0 ) ) ) );
	nodeVar226 = ( nodeVar225 * nodeVar204 );
	nodeVar227 = ( nodeVar218 * vec4<f32>( ( ( v_positionWorld + nodeVar226 ) + ( nodeVar225 * vec3<f32>( nodeVar196 ) ) ), 1.0 ) );
	nodeVar228 = max( nodeVar227.w, 0.0001 );
	nodeVar229 = ( ( ( clamp( mix( nodeVar214, vec2<f32>( ( ( ( nodeVar227.x / nodeVar228 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar227.y / nodeVar228 ) * -0.5 ) + 0.5 ) ), nodeVar215 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform53.zw ) + render.nodeUniform53.xy ) / render.nodeUniform58 );
	nodeVar230 = normalize( refract( nodeVar198, nodeVar213, ( 1.0 / max( ( nodeVar216 + nodeVar217 ), 1.0 ) ) ) );
	nodeVar231 = ( nodeVar230 * nodeVar204 );
	nodeVar232 = ( nodeVar218 * vec4<f32>( ( ( v_positionWorld + nodeVar231 ) + ( nodeVar230 * vec3<f32>( nodeVar196 ) ) ), 1.0 ) );
	nodeVar233 = max( nodeVar232.w, 0.0001 );
	nodeVar234 = ( ( ( clamp( mix( nodeVar214, vec2<f32>( ( ( ( nodeVar232.x / nodeVar233 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar232.y / nodeVar233 ) * -0.5 ) + 0.5 ) ), nodeVar215 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform53.zw ) + render.nodeUniform53.xy ) / render.nodeUniform58 );
	nodeVar235 = ( ( ( nodeVar214 * render.nodeUniform53.zw ) + render.nodeUniform53.xy ) / render.nodeUniform58 );
	nodeVar236 = vec2<f32>( textureDimensions( nodeUniform59, 0 ) );
	nodeVar237 = ( ( ( ( ( ( nodeVar224 + nodeVar196 ) * nodeVar210 ) / nodeVar216 ) * ( ( render.cameraProjectionMatrix[ 1u ][ 1u ] * render.nodeUniform58.y ) * 0.5 ) ) / nodeVar222 ) * object.nodeUniform60 );
	nodeVar238 = clamp( log2( max( nodeVar237, 1.0 ) ), 0.0, 0.0 );
	nodeVar239 = ( ( nodeVar223 - nodeVar235 ) * nodeVar236 );
	nodeVar240 = ( nodeVar239 / vec2<f32>( max( length( nodeVar239 ), 0.0001 ) ) );
	nodeVar241 = max( ( nodeVar238 - 1.403677461028802 ), 0.0 );
	nodeVar242 = nodeVar237;
	nodeVar243 = ( nodeVar212.x * 6.283185307179586 );
	nodeVar244 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	if ( ( nodeVar217 < 0.0001 ) ) {

		nodeVar245 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

		for ( var i : i32 = 0; i < 7; i ++ ) {

			nodeVar246 = ( ( f32( i ) * 2.399963229728653 ) + nodeVar243 );
			nodeVar247 = ( ( vec2<f32>( cos( nodeVar246 ), sin( nodeVar246 ) ) * vec2<f32>( sqrt( ( ( f32( i ) + 0.5 ) / 7.0 ) ) ) ) * vec2<f32>( nodeVar242 ) );
			nodeVar248 = ( ( nodeVar247 + ( ( ( nodeVar240 * vec2<f32>( dot( nodeVar247, nodeVar240 ) ) ) * vec2<f32>( nodeVar197 ) ) * vec2<f32>( 2.0 ) ) ) / nodeVar236 );
			nodeVar249 = textureSampleLevel( nodeUniform59, nodeUniform59_sampler, ( nodeVar223 + nodeVar248 ), nodeVar241 );
			nodeVar245 = ( nodeVar245 + nodeVar249 );

		}

		nodeVar244 = ( nodeVar245 / vec4<f32>( 7.0 ) );
		

	} else {

		nodeVar250 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
		nodeVar251 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
		nodeVar252 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

		for ( var i : i32 = 0; i < 7; i ++ ) {

			nodeVar253 = ( ( f32( i ) * 2.399963229728653 ) + nodeVar243 );
			nodeVar254 = ( ( vec2<f32>( cos( nodeVar253 ), sin( nodeVar253 ) ) * vec2<f32>( sqrt( ( ( f32( i ) + 0.5 ) / 7.0 ) ) ) ) * vec2<f32>( nodeVar242 ) );
			nodeVar255 = ( ( nodeVar254 + ( ( ( nodeVar240 * vec2<f32>( dot( nodeVar254, nodeVar240 ) ) ) * vec2<f32>( nodeVar197 ) ) * vec2<f32>( 2.0 ) ) ) / nodeVar236 );
			nodeVar256 = textureSampleLevel( nodeUniform59, nodeUniform59_sampler, ( nodeVar229 + nodeVar255 ), nodeVar241 );
			nodeVar250 = ( nodeVar250 + nodeVar256 );
			nodeVar257 = textureSampleLevel( nodeUniform59, nodeUniform59_sampler, ( nodeVar223 + nodeVar255 ), nodeVar241 );
			nodeVar251 = ( nodeVar251 + nodeVar257 );
			nodeVar258 = textureSampleLevel( nodeUniform59, nodeUniform59_sampler, ( nodeVar234 + nodeVar255 ), nodeVar241 );
			nodeVar252 = ( nodeVar252 + nodeVar258 );

		}

		nodeVar259 = ( nodeVar251 / vec4<f32>( 7.0 ) );
		nodeVar244 = vec4<f32>( ( nodeVar250 / vec4<f32>( 7.0 ) ).x, nodeVar259.y, ( nodeVar252 / vec4<f32>( 7.0 ) ).z, nodeVar259.w );
		

	}

	nodeVar260 = nodeVar244.xyz;
	nodeVar260 = ( nodeVar260 + ( ( object.nodeUniform61 * vec3<f32>( clamp( ( 1.0 - nodeVar244.w ), 0.0, 1.0 ) ) ) * vec3<f32>( object.nodeUniform62 ) ) );
	nodeVar261 = object.nodeUniform63;

	if ( ( ( nodeVar261 > 0.0 ) && ( nodeVar261 < 1e30 ) ) ) {

		nodeVar262 = exp( ( - ( ( ( - log( clamp( object.nodeUniform64, vec3<f32>( 0.0001 ), vec3<f32>( 1.0 ) ) ) ) / vec3<f32>( max( nodeVar261, 0.000001 ) ) ) * vec3<f32>( nodeVar224 ) ) ) );

	} else {

		nodeVar262 = vec3<f32>( 1.0, 1.0, 1.0 );

	}

	nodeVar260 = ( nodeVar260 * nodeVar262 );
	nodeVar263 = mix( nodeVar195, ( ( ( nodeVar260 * DiffuseColor.xyz ) * vec3<f32>( ( 1.0 - Metalness ) ) ) * ( vec3<f32>( 1.0 ) - ( SpecularColor + ( ( max( vec3<f32>( ( 1.0 - nodeVar209 ) ), SpecularColor ) - SpecularColor ) * vec3<f32>( pow( ( 1.0 - clamp( dot( nodeVar201, ( - nodeVar198 ) ), 0.0, 1.0 ) ), 5.0 ) ) ) ) ) ), object.nodeUniform65 );
	totalDiffuse = nodeVar263;
	nodeVar264 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar264;
	nodeVar265 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar265;
	nodeVar266 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar266;

	// result

	output.color = nodeVar266;

	return output;

}
