// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform11_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform11 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform27_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform27 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform46_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform46 : texture_2d<f32>;

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

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> nodeVar2 : f32;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> NORMAL_nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : f32;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec2<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : vec3<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : vec2<f32>;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : vec2<f32>;
var<private> nodeVar60 : vec4<f32>;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar63 : f32;
var<private> nodeVar64 : f32;
var<private> nodeVar65 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : vec2<f32>;
var<private> nodeVar70 : vec4<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : f32;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : f32;
var<private> nodeVar75 : f32;
var<private> nodeVar76 : f32;
var<private> nodeVar77 : vec2<f32>;
var<private> nodeVar78 : vec4<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : f32;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : f32;
var<private> nodeVar93 : f32;
var<private> nodeVar94 : f32;
var<private> nodeVar95 : vec3<f32>;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> nodeVar110 : f32;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : f32;
var<private> nodeVar119 : f32;
var<private> nodeVar120 : f32;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : f32;
var<private> nodeVar129 : vec3<f32>;
var<private> nodeVar130 : vec3<f32>;
var<private> nodeVar131 : vec3<f32>;
var<private> nodeVar132 : vec3<f32>;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : f32;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : vec3<f32>;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec3<f32>;
var<private> nodeVar142 : vec3<f32>;
var<private> nodeVar143 : vec3<f32>;
var<private> nodeVar144 : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> nodeVar151 : vec3<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : vec3<f32>;
var<private> nodeVar156 : vec3<f32>;
var<private> nodeVar157 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar158 : vec3<f32>;
var<private> nodeVar159 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar160 : vec3<f32>;
var<private> nodeVar161 : f32;
var<private> nodeVar162 : f32;
var<private> nodeVar163 : f32;
var<private> nodeVar164 : f32;
var<private> nodeVar165 : f32;
var<private> nodeVar166 : f32;
var<private> nodeVar167 : f32;
var<private> nodeVar168 : f32;
var<private> nodeVar169 : f32;
var<private> nodeVar170 : f32;
var<private> nodeVar171 : f32;
var<private> nodeVar172 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar173 : vec3<f32>;
var<private> nodeVar174 : f32;
var<private> nodeVar175 : f32;
var<private> nodeVar176 : vec3<f32>;
var<private> nodeVar177 : vec3<f32>;
var<private> nodeVar178 : f32;
var<private> nodeVar179 : vec3<f32>;
var<private> nodeVar180 : vec3<f32>;
var<private> nodeVar181 : mat3x3<f32>;
var<private> nodeVar182 : vec3<f32>;
var<private> nodeVar183 : vec3<f32>;
var<private> nodeVar184 : vec3<f32>;
var<private> nodeVar185 : vec3<f32>;
var<private> nodeVar186 : f32;
var<private> nodeVar187 : f32;
var<private> nodeVar188 : f32;
var<private> nodeVar189 : vec3<f32>;
var<private> normalWorldGeometry : vec3<f32>;
var<private> nodeVar190 : vec3<f32>;
var<private> nodeVar191 : vec4<f32>;
var<private> nodeVar192 : vec3<f32>;
var<private> nodeVar193 : vec2<f32>;
var<private> nodeVar194 : f32;
var<private> nodeVar195 : f32;
var<private> nodeVar196 : f32;
var<private> nodeVar197 : mat4x4<f32>;
var<private> nodeVar198 : vec3<f32>;
var<private> nodeVar199 : vec3<f32>;
var<private> nodeVar200 : vec4<f32>;
var<private> nodeVar201 : f32;
var<private> nodeVar202 : vec2<f32>;
var<private> nodeVar203 : f32;
var<private> nodeVar204 : vec3<f32>;
var<private> nodeVar205 : vec3<f32>;
var<private> nodeVar206 : vec4<f32>;
var<private> nodeVar207 : f32;
var<private> nodeVar208 : vec2<f32>;
var<private> nodeVar209 : vec3<f32>;
var<private> nodeVar210 : vec3<f32>;
var<private> nodeVar211 : vec4<f32>;
var<private> nodeVar212 : f32;
var<private> nodeVar213 : vec2<f32>;
var<private> nodeVar214 : vec2<f32>;
var<private> nodeVar215 : vec2<f32>;
var<private> nodeVar216 : f32;
var<private> nodeVar217 : f32;
var<private> nodeVar218 : vec2<f32>;
var<private> nodeVar219 : vec2<f32>;
var<private> nodeVar220 : f32;
var<private> nodeVar221 : vec2<f32>;
var<private> nodeVar222 : vec2<f32>;
var<private> nodeVar223 : vec2<f32>;
var<private> nodeVar224 : vec4<f32>;
var<private> nodeVar225 : vec2<f32>;
var<private> nodeVar226 : vec2<f32>;
var<private> nodeVar227 : vec2<f32>;
var<private> nodeVar228 : vec2<f32>;
var<private> nodeVar229 : vec2<f32>;
var<private> nodeVar230 : vec2<f32>;
var<private> nodeVar231 : vec2<f32>;
var<private> nodeVar232 : vec4<f32>;
var<private> nodeVar233 : vec4<f32>;
var<private> nodeVar234 : vec4<f32>;
var<private> nodeVar235 : vec4<f32>;
var<private> nodeVar236 : vec4<f32>;
var<private> nodeVar237 : vec4<f32>;
var<private> nodeVar238 : vec4<f32>;
var<private> nodeVar239 : vec2<f32>;
var<private> nodeVar240 : vec2<f32>;
var<private> nodeVar241 : vec2<f32>;
var<private> nodeVar242 : vec2<f32>;
var<private> nodeVar243 : vec2<f32>;
var<private> nodeVar244 : vec2<f32>;
var<private> nodeVar245 : vec2<f32>;
var<private> nodeVar246 : vec4<f32>;
var<private> nodeVar247 : vec4<f32>;
var<private> nodeVar248 : vec4<f32>;
var<private> nodeVar249 : vec4<f32>;
var<private> nodeVar250 : vec4<f32>;
var<private> nodeVar251 : vec4<f32>;
var<private> nodeVar252 : vec4<f32>;
var<private> nodeVar253 : vec2<f32>;
var<private> nodeVar254 : vec2<f32>;
var<private> nodeVar255 : vec2<f32>;
var<private> nodeVar256 : vec2<f32>;
var<private> nodeVar257 : vec2<f32>;
var<private> nodeVar258 : vec2<f32>;
var<private> nodeVar259 : vec2<f32>;
var<private> nodeVar260 : vec4<f32>;
var<private> nodeVar261 : vec4<f32>;
var<private> nodeVar262 : vec4<f32>;
var<private> nodeVar263 : vec4<f32>;
var<private> nodeVar264 : vec4<f32>;
var<private> nodeVar265 : vec4<f32>;
var<private> nodeVar266 : vec4<f32>;
var<private> nodeVar267 : vec2<f32>;
var<private> nodeVar268 : vec2<f32>;
var<private> nodeVar269 : vec2<f32>;
var<private> nodeVar270 : vec2<f32>;
var<private> nodeVar271 : vec2<f32>;
var<private> nodeVar272 : vec2<f32>;
var<private> nodeVar273 : vec2<f32>;
var<private> nodeVar274 : vec4<f32>;
var<private> nodeVar275 : vec4<f32>;
var<private> nodeVar276 : vec4<f32>;
var<private> nodeVar277 : vec4<f32>;
var<private> nodeVar278 : vec4<f32>;
var<private> nodeVar279 : vec4<f32>;
var<private> nodeVar280 : vec4<f32>;
var<private> nodeVar281 : vec3<f32>;
var<private> nodeVar282 : f32;
var<private> nodeVar283 : vec3<f32>;
var<private> nodeVar284 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar285 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar286 : vec3<f32>;
var<private> nodeVar287 : vec4<f32>;
var<private> nodeVar288 : vec3<f32>;

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
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_normalWorldGeometry : vec3<f32>,
	@builtin( front_facing ) isFront : bool,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	Metalness = object.nodeUniform2;
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar1 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform3, 0.0525 ) + max( max( nodeVar1.x, nodeVar1.y ), nodeVar1.z ) ), 1.0 );
	IOR = object.nodeUniform6;
	nodeVar2 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	SpecularColor = ( min( ( vec3<f32>( ( nodeVar2 * nodeVar2 ) ) * object.nodeUniform7 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform8 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform8, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform2 ) ) );
	EmissiveColor = ( object.nodeUniform9 * vec3<f32>( object.nodeUniform10 ) );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) );
	NORMAL_nodeVar3 = NORMAL_normalView;
	nodeVar4 = ( v_positionWorld * vec3<f32>( object.nodeUniform13 ) );
	nodeVar5 = fwidth( nodeVar4 );
	nodeVar6 = ( 1.0 - smoothstep( 0.25, 0.75, max( max( nodeVar5.x, nodeVar5.y ), nodeVar5.z ) ) );

	if ( ( object.nodeUniform14 > 0.00001 ) ) {

		NORMAL_nodeVar3 = normalize( ( NORMAL_nodeVar3 + ( ( ( render.cameraViewMatrix * vec4<f32>( ( ( mx_perlin_noise_vec3_1( ( nodeVar4 + ( vec3<f32>( 1.0, -1.0, -1.0 ) * vec3<f32>( ( object.nodeUniform16 * object.nodeUniform17 ) ) ) ) ) * vec3<f32>( 1.0 ) ) + vec3<f32>( 0.0 ) ), 0.0 ) ).xyz * vec3<f32>( object.nodeUniform14 ) ) * vec3<f32>( nodeVar6 ) ) ) );
		

	}

	normalView = NORMAL_nodeVar3;
	positionViewDirection = vec3<f32>( 0.0, 0.0, 1.0 );
	nodeVar7 = dot( normalView, positionViewDirection );
	nodeVar8 = textureSample( nodeUniform11, nodeUniform11_sampler, vec2<f32>( Roughness, clamp( nodeVar7, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar8;
	nodeVar9 = ( dfg.x + dfg.y );
	nodeVar10 = ( 1.0 / nodeVar9 );
	nodeVar11 = nodeVar10;
	nodeVar12 = ( nodeVar11 - 1.0 );
	nodeVar13 = ( SpecularColorBlended * vec3<f32>( nodeVar12 ) );
	nodeVar14 = ( nodeVar13 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar14;
	nodeVar15 = ( render.nodeUniform18 - render.nodeUniform19 );
	nodeVar16 = vec4<f32>( nodeVar15, 0.0 );
	nodeVar17 = ( render.cameraViewMatrix * nodeVar16 );
	nodeVar18 = normalize( nodeVar17.xyz );
	nodeVar19 = nodeVar18;
	nodeVar20 = dot( normalView, nodeVar19 );
	nodeVar21 = ( vec3<f32>( clamp( nodeVar20, 0.0, 1.0 ) ) * render.nodeUniform20 );
	nodeVar22 = nodeVar21;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar23 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar24 = ( nodeVar22 * nodeVar23 );
	nodeVar25 = ( nodeVar19 + positionViewDirection );
	nodeVar26 = normalize( nodeVar25 );
	nodeVar27 = dot( positionViewDirection, nodeVar26 );
	nodeVar28 = clamp( nodeVar27, 0.0, 1.0 );
	nodeVar29 = exp2( ( ( ( nodeVar28 * -5.55473 ) - 6.98316 ) * nodeVar28 ) );
	nodeVar30 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar29 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar29 ) ) );
	nodeVar31 = ( vec3<f32>( 1.0 ) - nodeVar30 );
	nodeVar32 = nodeVar31;
	nodeVar33 = ( nodeVar24 * nodeVar32 );
	nodeVar34 = ( directDiffuse + nodeVar33 );
	directDiffuse = nodeVar34;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar35 = normalize( ( nodeVar19 + positionViewDirection ) );
	nodeVar36 = clamp( dot( positionViewDirection, nodeVar35 ), 0.0, 1.0 );
	nodeVar37 = exp2( ( ( ( nodeVar36 * -5.55473 ) - 6.98316 ) * nodeVar36 ) );
	nodeVar38 = ( Roughness * Roughness );
	nodeVar39 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar37 ) ) ) + vec3<f32>( ( 1.0 * nodeVar37 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar38, clamp( dot( normalView, nodeVar19 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar38, clamp( dot( normalView, nodeVar35 ), 0.0, 1.0 ) ) ) );
	nodeVar40 = ( nodeVar22 * nodeVar39 );
	nodeVar41 = ( nodeVar40 * multiScatteringCompensation );
	nodeVar42 = ( directSpecular + nodeVar41 );
	directSpecular = nodeVar42;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar43 = ( irradiance + render.nodeUniform21 );
	irradiance = nodeVar43;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar44 = clamp( roughnessToMip( Roughness ), -2.0, object.nodeUniform22 );
	nodeVar45 = floor( nodeVar44 );
	nodeVar46 = nodeVar45;
	nodeVar47 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	nodeVar48 = getFace( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( nodeVar47.x, ( - nodeVar47.y ), nodeVar47.z ), 1.0 ) ).xyz );
	nodeVar49 = max( ( 4.0 - nodeVar46 ), 0.0 );
	nodeVar46 = max( nodeVar46, 4.0 );
	nodeVar50 = exp2( nodeVar46 );
	nodeVar51 = ( ( getUV( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( nodeVar47.x, ( - nodeVar47.y ), nodeVar47.z ), 1.0 ) ).xyz, nodeVar48 ) * vec2<f32>( ( nodeVar50 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar48 > 2.0 ) ) {

		nodeVar51.y = ( nodeVar51.y + nodeVar50 );
		nodeVar48 = ( nodeVar48 - 3.0 );
		

	}

	nodeVar51.x = ( nodeVar51.x + ( nodeVar48 * nodeVar50 ) );
	nodeVar51.x = ( nodeVar51.x + ( nodeVar49 * ( 3.0 * 16.0 ) ) );
	nodeVar51.y = ( nodeVar51.y + ( 4.0 * ( exp2( object.nodeUniform22 ) - nodeVar50 ) ) );
	nodeVar51.x = ( nodeVar51.x * object.nodeUniform25 );
	nodeVar51.y = ( nodeVar51.y * object.nodeUniform26 );
	nodeVar52 = textureSampleGrad( nodeUniform27, nodeUniform27_sampler, nodeVar51, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar53 = nodeVar52.xyz;
	nodeVar54 = fract( nodeVar44 );

	if ( ( nodeVar54 != 0.0 ) ) {

		nodeVar55 = ( nodeVar45 + 1.0 );
		nodeVar56 = getFace( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( nodeVar47.x, ( - nodeVar47.y ), nodeVar47.z ), 1.0 ) ).xyz );
		nodeVar57 = max( ( 4.0 - nodeVar55 ), 0.0 );
		nodeVar55 = max( nodeVar55, 4.0 );
		nodeVar58 = exp2( nodeVar55 );
		nodeVar59 = ( ( getUV( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( nodeVar47.x, ( - nodeVar47.y ), nodeVar47.z ), 1.0 ) ).xyz, nodeVar56 ) * vec2<f32>( ( nodeVar58 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar56 > 2.0 ) ) {

			nodeVar59.y = ( nodeVar59.y + nodeVar58 );
			nodeVar56 = ( nodeVar56 - 3.0 );
			

		}

		nodeVar59.x = ( nodeVar59.x + ( nodeVar56 * nodeVar58 ) );
		nodeVar59.x = ( nodeVar59.x + ( nodeVar57 * ( 3.0 * 16.0 ) ) );
		nodeVar59.y = ( nodeVar59.y + ( 4.0 * ( exp2( object.nodeUniform22 ) - nodeVar58 ) ) );
		nodeVar59.x = ( nodeVar59.x * object.nodeUniform25 );
		nodeVar59.y = ( nodeVar59.y * object.nodeUniform26 );
		nodeVar60 = textureSampleGrad( nodeUniform27, nodeUniform27_sampler, nodeVar59, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar61 = nodeVar60.xyz;
		nodeVar53 = mix( nodeVar53, nodeVar61, nodeVar54 );
		

	}

	nodeVar62 = ( radiance + ( nodeVar53 * vec3<f32>( object.nodeUniform28 ) ) );
	radiance = nodeVar62;
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar63 = clamp( roughnessToMip( 1.0 ), -2.0, object.nodeUniform22 );
	nodeVar64 = floor( nodeVar63 );
	nodeVar65 = nodeVar64;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar66 = getFace( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
	nodeVar67 = max( ( 4.0 - nodeVar65 ), 0.0 );
	nodeVar65 = max( nodeVar65, 4.0 );
	nodeVar68 = exp2( nodeVar65 );
	nodeVar69 = ( ( getUV( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar66 ) * vec2<f32>( ( nodeVar68 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar66 > 2.0 ) ) {

		nodeVar69.y = ( nodeVar69.y + nodeVar68 );
		nodeVar66 = ( nodeVar66 - 3.0 );
		

	}

	nodeVar69.x = ( nodeVar69.x + ( nodeVar66 * nodeVar68 ) );
	nodeVar69.x = ( nodeVar69.x + ( nodeVar67 * ( 3.0 * 16.0 ) ) );
	nodeVar69.y = ( nodeVar69.y + ( 4.0 * ( exp2( object.nodeUniform22 ) - nodeVar68 ) ) );
	nodeVar69.x = ( nodeVar69.x * object.nodeUniform25 );
	nodeVar69.y = ( nodeVar69.y * object.nodeUniform26 );
	nodeVar70 = textureSampleGrad( nodeUniform27, nodeUniform27_sampler, nodeVar69, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar71 = nodeVar70.xyz;
	nodeVar72 = fract( nodeVar63 );

	if ( ( nodeVar72 != 0.0 ) ) {

		nodeVar73 = ( nodeVar64 + 1.0 );
		nodeVar74 = getFace( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
		nodeVar75 = max( ( 4.0 - nodeVar73 ), 0.0 );
		nodeVar73 = max( nodeVar73, 4.0 );
		nodeVar76 = exp2( nodeVar73 );
		nodeVar77 = ( ( getUV( ( object.nodeUniform23 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar74 ) * vec2<f32>( ( nodeVar76 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar74 > 2.0 ) ) {

			nodeVar77.y = ( nodeVar77.y + nodeVar76 );
			nodeVar74 = ( nodeVar74 - 3.0 );
			

		}

		nodeVar77.x = ( nodeVar77.x + ( nodeVar74 * nodeVar76 ) );
		nodeVar77.x = ( nodeVar77.x + ( nodeVar75 * ( 3.0 * 16.0 ) ) );
		nodeVar77.y = ( nodeVar77.y + ( 4.0 * ( exp2( object.nodeUniform22 ) - nodeVar76 ) ) );
		nodeVar77.x = ( nodeVar77.x * object.nodeUniform25 );
		nodeVar77.y = ( nodeVar77.y * object.nodeUniform26 );
		nodeVar78 = textureSampleGrad( nodeUniform27, nodeUniform27_sampler, nodeVar77, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar79 = nodeVar78.xyz;
		nodeVar71 = mix( nodeVar71, nodeVar79, nodeVar72 );
		

	}

	nodeVar80 = ( iblIrradiance + ( ( nodeVar71 * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform28 ) ) );
	iblIrradiance = nodeVar80;
	nodeVar81 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar82 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar83 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar84 = ( SpecularF90 * dfg.y );
	nodeVar85 = ( nodeVar83 + vec3<f32>( nodeVar84 ) );
	nodeVar86 = ( nodeVar81 + nodeVar85 );
	nodeVar81 = nodeVar86;
	nodeVar87 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar88 = nodeVar87;
	nodeVar89 = ( nodeVar88 * vec3<f32>( 0.047619 ) );
	nodeVar90 = ( SpecularColor + nodeVar89 );
	nodeVar91 = ( nodeVar85 * nodeVar90 );
	nodeVar92 = ( dfg.x + dfg.y );
	nodeVar93 = ( 1.0 - nodeVar92 );
	nodeVar94 = nodeVar93;
	nodeVar95 = ( vec3<f32>( nodeVar94 ) * nodeVar90 );
	nodeVar96 = ( vec3<f32>( 1.0 ) - nodeVar95 );
	nodeVar97 = nodeVar96;
	nodeVar98 = ( nodeVar91 / nodeVar97 );
	nodeVar99 = ( nodeVar98 * vec3<f32>( nodeVar94 ) );
	nodeVar100 = ( nodeVar82 + nodeVar99 );
	nodeVar82 = nodeVar100;
	nodeVar101 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar102 = ( irradiance * nodeVar101 );
	nodeVar103 = ( nodeVar81 + nodeVar82 );
	nodeVar104 = ( vec3<f32>( 1.0 ) - nodeVar103 );
	nodeVar105 = nodeVar104;
	nodeVar106 = ( nodeVar102 * nodeVar105 );
	nodeVar107 = nodeVar106;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar108 = ( indirectDiffuse + nodeVar107 );
	indirectDiffuse = nodeVar108;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar109 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar110 = ( SpecularF90 * dfg.y );
	nodeVar111 = ( nodeVar109 + vec3<f32>( nodeVar110 ) );
	nodeVar112 = ( singleScatteringDielectric + nodeVar111 );
	singleScatteringDielectric = nodeVar112;
	nodeVar113 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar114 = nodeVar113;
	nodeVar115 = ( nodeVar114 * vec3<f32>( 0.047619 ) );
	nodeVar116 = ( SpecularColor + nodeVar115 );
	nodeVar117 = ( nodeVar111 * nodeVar116 );
	nodeVar118 = ( dfg.x + dfg.y );
	nodeVar119 = ( 1.0 - nodeVar118 );
	nodeVar120 = nodeVar119;
	nodeVar121 = ( vec3<f32>( nodeVar120 ) * nodeVar116 );
	nodeVar122 = ( vec3<f32>( 1.0 ) - nodeVar121 );
	nodeVar123 = nodeVar122;
	nodeVar124 = ( nodeVar117 / nodeVar123 );
	nodeVar125 = ( nodeVar124 * vec3<f32>( nodeVar120 ) );
	nodeVar126 = ( multiScatteringDielectric + nodeVar125 );
	multiScatteringDielectric = nodeVar126;
	nodeVar127 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar128 = ( SpecularF90 * dfg.y );
	nodeVar129 = ( nodeVar127 + vec3<f32>( nodeVar128 ) );
	nodeVar130 = ( singleScatteringMetallic + nodeVar129 );
	singleScatteringMetallic = nodeVar130;
	nodeVar131 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar132 = nodeVar131;
	nodeVar133 = ( nodeVar132 * vec3<f32>( 0.047619 ) );
	nodeVar134 = ( DiffuseColor.xyz + nodeVar133 );
	nodeVar135 = ( nodeVar129 * nodeVar134 );
	nodeVar136 = ( dfg.x + dfg.y );
	nodeVar137 = ( 1.0 - nodeVar136 );
	nodeVar138 = nodeVar137;
	nodeVar139 = ( vec3<f32>( nodeVar138 ) * nodeVar134 );
	nodeVar140 = ( vec3<f32>( 1.0 ) - nodeVar139 );
	nodeVar141 = nodeVar140;
	nodeVar142 = ( nodeVar135 / nodeVar141 );
	nodeVar143 = ( nodeVar142 * vec3<f32>( nodeVar138 ) );
	nodeVar144 = ( multiScatteringMetallic + nodeVar143 );
	multiScatteringMetallic = nodeVar144;
	nodeVar145 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar146 = ( radiance * nodeVar145 );
	nodeVar147 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	nodeVar148 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar149 = ( nodeVar147 * nodeVar148 );
	nodeVar150 = ( nodeVar146 + nodeVar149 );
	nodeVar151 = nodeVar150;
	nodeVar152 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar153 = ( vec3<f32>( 1.0 ) - nodeVar152 );
	nodeVar154 = nodeVar153;
	nodeVar155 = ( DiffuseContribution * nodeVar154 );
	nodeVar156 = ( nodeVar155 * nodeVar148 );
	nodeVar157 = nodeVar156;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar158 = ( indirectSpecular + nodeVar151 );
	indirectSpecular = nodeVar158;
	nodeVar159 = ( indirectDiffuse + nodeVar157 );
	indirectDiffuse = nodeVar159;
	ambientOcclusion = 1.0;
	nodeVar160 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar160;
	nodeVar161 = dot( normalView, positionViewDirection );
	nodeVar162 = ( clamp( nodeVar161, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar163 = ( Roughness * -16.0 );
	nodeVar164 = ( 1.0 - nodeVar163 );
	nodeVar165 = nodeVar164;
	nodeVar166 = ( - nodeVar165 );
	nodeVar167 = exp2( nodeVar166 );
	nodeVar168 = pow( nodeVar162, nodeVar167 );
	nodeVar169 = ( 1.0 - nodeVar168 );
	nodeVar170 = nodeVar169;
	nodeVar171 = ( ambientOcclusion - nodeVar170 );
	nodeVar172 = ( indirectSpecular * vec3<f32>( clamp( nodeVar171, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar172;
	nodeVar173 = ( directDiffuse + indirectDiffuse );
	nodeVar174 = object.nodeUniform29;
	nodeVar175 = object.nodeUniform30;
	nodeVar176 = normalize( ( v_positionWorld - render.cameraPosition ) );
	nodeVar177 = normalWorld;

	if ( ( dot( nodeVar177, nodeVar176 ) > 0.0 ) ) {

		nodeVar178 = -1.0;

	} else {

		nodeVar178 = 1.0;

	}

	nodeVar179 = ( nodeVar177 * vec3<f32>( nodeVar178 ) );
	nodeVar180 = vec3<f32>( length( object.nodeUniform32[ 0u ].xyz ), length( object.nodeUniform32[ 1u ].xyz ), length( object.nodeUniform32[ 2u ].xyz ) );
	nodeVar181 = mat3x3<f32>( ( object.nodeUniform32[ 0u ].xyz / vec3<f32>( nodeVar180.x ) ), ( object.nodeUniform32[ 1u ].xyz / vec3<f32>( nodeVar180.y ) ), ( object.nodeUniform32[ 2u ].xyz / vec3<f32>( nodeVar180.z ) ) );
	nodeVar182 = ( nodeVar180 * vec3<f32>( object.nodeUniform33 ) );
	nodeVar183 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar184 = ( v_positionWorld * vec3<f32>( object.nodeUniform34 ) );
	nodeVar185 = fwidth( nodeVar184 );
	nodeVar186 = ( 1.0 - smoothstep( 0.25, 0.75, max( max( nodeVar185.x, nodeVar185.y ), nodeVar185.z ) ) );

	if ( ( object.nodeUniform35 > 0.00001 ) ) {

		nodeVar183 = ( ( ( nodeVar181 * ( ( mx_perlin_noise_vec3_1( ( nodeVar184 + vec3<f32>( ( object.nodeUniform36 * object.nodeUniform37 ) ) ) ) * vec3<f32>( 1.0 ) ) + vec3<f32>( 0.0 ) ) ) * vec3<f32>( object.nodeUniform35 ) ) * vec3<f32>( nodeVar186 ) );
		

	}

	nodeVar187 = clamp( object.nodeUniform3, 0.0, 1.0 );
	nodeVar188 = ( nodeVar187 * nodeVar187 );
	nodeVar189 = floor( ( v_positionWorld * vec3<f32>( object.nodeUniform38 ) ) );
	normalWorldGeometry = normalize( v_normalWorldGeometry );
	nodeVar190 = clamp( ( ( abs( normalWorldGeometry ) - vec3<f32>( 0.5773 ) ) / vec3<f32>( 0.42269999999999996 ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) );
	nodeVar191 = ( ( ( ( vec4<f32>( interleavedGradientNoise( nodeVar189.yz ), interleavedGradientNoise( ( nodeVar189.yz + vec2<f32>( 47.0, 17.0 ) ) ), interleavedGradientNoise( ( nodeVar189.yz + vec2<f32>( -23.0, 71.0 ) ) ), interleavedGradientNoise( ( nodeVar189.yz + vec2<f32>( 89.0, -31.0 ) ) ) ) * vec4<f32>( nodeVar190.x ) ) + ( vec4<f32>( interleavedGradientNoise( nodeVar189.zx ), interleavedGradientNoise( ( nodeVar189.zx + vec2<f32>( 47.0, 17.0 ) ) ), interleavedGradientNoise( ( nodeVar189.zx + vec2<f32>( -23.0, 71.0 ) ) ), interleavedGradientNoise( ( nodeVar189.zx + vec2<f32>( 89.0, -31.0 ) ) ) ) * vec4<f32>( nodeVar190.y ) ) ) + ( vec4<f32>( interleavedGradientNoise( nodeVar189.xy ), interleavedGradientNoise( ( nodeVar189.xy + vec2<f32>( 47.0, 17.0 ) ) ), interleavedGradientNoise( ( nodeVar189.xy + vec2<f32>( -23.0, 71.0 ) ) ), interleavedGradientNoise( ( nodeVar189.xy + vec2<f32>( 89.0, -31.0 ) ) ) ) * vec4<f32>( nodeVar190.z ) ) ) / vec4<f32>( max( ( ( nodeVar190.x + nodeVar190.y ) + nodeVar190.z ), 0.0001 ) ) );
	nodeVar192 = normalize( ( ( nodeVar179 + ( ( ( ( ( nodeVar191.xyw - vec3<f32>( 0.5 ) ) * vec3<f32>( 2.0 ) ) * vec3<f32>( nodeVar188 ) ) * vec3<f32>( object.nodeUniform39 ) ) * vec3<f32>( 0.35 ) ) ) + nodeVar183 ) );
	nodeVar193 = ( ( fragCoord.xy - render.nodeUniform40.xy ) / render.nodeUniform40.zw );
	nodeVar194 = smoothstep( 0.0, 1.0, ( min( min( nodeVar193.x, ( 1.0 - nodeVar193.x ) ), min( nodeVar193.y, ( 1.0 - nodeVar193.y ) ) ) / max( object.nodeUniform41, 0.00001 ) ) );
	nodeVar195 = max( IOR, 1.0 );
	nodeVar196 = ( max( ( nodeVar195 - 1.0 ), 0.0 ) * ( ( object.nodeUniform42 * 0.025 ) + object.nodeUniform43 ) );
	nodeVar197 = ( render.cameraProjectionMatrix * render.cameraViewMatrix );
	nodeVar198 = normalize( refract( nodeVar176, nodeVar192, ( 1.0 / max( nodeVar195, 1.0 ) ) ) );
	nodeVar199 = ( nodeVar198 * nodeVar182 );
	nodeVar200 = ( nodeVar197 * vec4<f32>( ( ( v_positionWorld + nodeVar199 ) + ( nodeVar198 * vec3<f32>( nodeVar174 ) ) ), 1.0 ) );
	nodeVar201 = max( nodeVar200.w, 0.0001 );
	nodeVar202 = ( ( ( clamp( mix( nodeVar193, vec2<f32>( ( ( ( nodeVar200.x / nodeVar201 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar200.y / nodeVar201 ) * -0.5 ) + 0.5 ) ), nodeVar194 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform40.zw ) + render.nodeUniform40.xy ) / render.nodeUniform45 );
	nodeVar203 = length( nodeVar199 );
	nodeVar204 = normalize( refract( nodeVar176, nodeVar192, ( 1.0 / max( ( nodeVar195 - nodeVar196 ), 1.0 ) ) ) );
	nodeVar205 = ( nodeVar204 * nodeVar182 );
	nodeVar206 = ( nodeVar197 * vec4<f32>( ( ( v_positionWorld + nodeVar205 ) + ( nodeVar204 * vec3<f32>( nodeVar174 ) ) ), 1.0 ) );
	nodeVar207 = max( nodeVar206.w, 0.0001 );
	nodeVar208 = ( ( ( clamp( mix( nodeVar193, vec2<f32>( ( ( ( nodeVar206.x / nodeVar207 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar206.y / nodeVar207 ) * -0.5 ) + 0.5 ) ), nodeVar194 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform40.zw ) + render.nodeUniform40.xy ) / render.nodeUniform45 );
	nodeVar209 = normalize( refract( nodeVar176, nodeVar192, ( 1.0 / max( ( nodeVar195 + nodeVar196 ), 1.0 ) ) ) );
	nodeVar210 = ( nodeVar209 * nodeVar182 );
	nodeVar211 = ( nodeVar197 * vec4<f32>( ( ( v_positionWorld + nodeVar210 ) + ( nodeVar209 * vec3<f32>( nodeVar174 ) ) ), 1.0 ) );
	nodeVar212 = max( nodeVar211.w, 0.0001 );
	nodeVar213 = ( ( ( clamp( mix( nodeVar193, vec2<f32>( ( ( ( nodeVar211.x / nodeVar212 ) * 0.5 ) + 0.5 ), ( ( ( nodeVar211.y / nodeVar212 ) * -0.5 ) + 0.5 ) ), nodeVar194 ), vec2<f32>( 0.001 ), vec2<f32>( 0.999 ) ) * render.nodeUniform40.zw ) + render.nodeUniform40.xy ) / render.nodeUniform45 );
	nodeVar214 = ( ( ( nodeVar193 * render.nodeUniform40.zw ) + render.nodeUniform40.xy ) / render.nodeUniform45 );
	nodeVar215 = vec2<f32>( textureDimensions( nodeUniform46, 0 ) );
	nodeVar216 = ( ( ( ( ( ( nodeVar203 + nodeVar174 ) * nodeVar188 ) / nodeVar195 ) * ( ( render.cameraProjectionMatrix[ 1u ][ 1u ] * render.nodeUniform45.y ) * 0.5 ) ) / nodeVar201 ) * object.nodeUniform47 );
	nodeVar217 = clamp( log2( max( nodeVar216, 1.0 ) ), 0.0, 0.0 );
	nodeVar218 = ( ( nodeVar202 - nodeVar214 ) * nodeVar215 );
	nodeVar219 = ( nodeVar218 / vec2<f32>( max( length( nodeVar218 ), 0.0001 ) ) );
	nodeVar220 = min( floor( ( nodeVar217 + nodeVar191.w ) ), 0.0 );
	nodeVar221 = max( floor( ( nodeVar215 / vec2<f32>( pow( 2.0, nodeVar220 ) ) ) ), vec2<f32>( 1.0, 1.0 ) );
	nodeVar222 = ( ( ( nodeVar191.yz - vec2<f32>( 0.5 ) ) * vec2<f32>( min( nodeVar217, 1.0 ) ) ) / nodeVar221 );
	nodeVar223 = ( ( ( nodeVar219 * vec2<f32>( nodeVar216 ) ) * vec2<f32>( nodeVar175 ) ) / nodeVar215 );
	nodeVar224 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	if ( ( nodeVar196 < 0.0001 ) ) {

		nodeVar225 = ( ( ( nodeVar202 + nodeVar222 ) * nodeVar221 ) + vec2<f32>( 0.5 ) );
		nodeVar226 = floor( nodeVar225 );
		nodeVar227 = fract( nodeVar225 );
		nodeVar228 = ( ( ( ( nodeVar227 * ( ( nodeVar227 * ( ( - nodeVar227 ) + vec2<f32>( 3.0 ) ) ) - vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( ( nodeVar227 * nodeVar227 ) * ( ( nodeVar227 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) );
		nodeVar229 = ( ( ( ( nodeVar227 * ( ( nodeVar227 * ( ( nodeVar227 * vec2<f32>( -3.0 ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( nodeVar227 * nodeVar227 ) * nodeVar227 ) / vec2<f32>( 6.0 ) ) );
		nodeVar230 = ( ( ( ( ( ( nodeVar227 * nodeVar227 ) * ( ( nodeVar227 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) / nodeVar228 ) - vec2<f32>( 1.0 ) );
		nodeVar231 = ( ( ( ( ( nodeVar227 * nodeVar227 ) * nodeVar227 ) / vec2<f32>( 6.0 ) ) / nodeVar229 ) + vec2<f32>( 1.0 ) );
		nodeVar232 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar226 + vec2<f32>( nodeVar230.x, nodeVar230.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar233 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar226 + vec2<f32>( nodeVar231.x, nodeVar230.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar234 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar226 + vec2<f32>( nodeVar230.x, nodeVar231.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar235 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar226 + vec2<f32>( nodeVar231.x, nodeVar231.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar236 = ( ( ( ( nodeVar232 * vec4<f32>( nodeVar228.x ) ) + ( nodeVar233 * vec4<f32>( nodeVar229.x ) ) ) * vec4<f32>( nodeVar228.y ) ) + ( ( ( nodeVar234 * vec4<f32>( nodeVar228.x ) ) + ( nodeVar235 * vec4<f32>( nodeVar229.x ) ) ) * vec4<f32>( nodeVar229.y ) ) );

		if ( ( nodeVar175 > 0.0001 ) ) {

			nodeVar237 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar202 + nodeVar223 ), nodeVar220 );
			nodeVar238 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar202 - nodeVar223 ), nodeVar220 );
			nodeVar236 = ( ( ( nodeVar236 + nodeVar237 ) + nodeVar238 ) / vec4<f32>( 3.0 ) );
			

		}

		nodeVar224 = nodeVar236;
		

	} else {

		nodeVar239 = ( ( ( nodeVar208 + nodeVar222 ) * nodeVar221 ) + vec2<f32>( 0.5 ) );
		nodeVar240 = floor( nodeVar239 );
		nodeVar241 = fract( nodeVar239 );
		nodeVar242 = ( ( ( ( nodeVar241 * ( ( nodeVar241 * ( ( - nodeVar241 ) + vec2<f32>( 3.0 ) ) ) - vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( ( nodeVar241 * nodeVar241 ) * ( ( nodeVar241 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) );
		nodeVar243 = ( ( ( ( nodeVar241 * ( ( nodeVar241 * ( ( nodeVar241 * vec2<f32>( -3.0 ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( nodeVar241 * nodeVar241 ) * nodeVar241 ) / vec2<f32>( 6.0 ) ) );
		nodeVar244 = ( ( ( ( ( ( nodeVar241 * nodeVar241 ) * ( ( nodeVar241 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) / nodeVar242 ) - vec2<f32>( 1.0 ) );
		nodeVar245 = ( ( ( ( ( nodeVar241 * nodeVar241 ) * nodeVar241 ) / vec2<f32>( 6.0 ) ) / nodeVar243 ) + vec2<f32>( 1.0 ) );
		nodeVar246 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar240 + vec2<f32>( nodeVar244.x, nodeVar244.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar247 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar240 + vec2<f32>( nodeVar245.x, nodeVar244.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar248 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar240 + vec2<f32>( nodeVar244.x, nodeVar245.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar249 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar240 + vec2<f32>( nodeVar245.x, nodeVar245.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar250 = ( ( ( ( nodeVar246 * vec4<f32>( nodeVar242.x ) ) + ( nodeVar247 * vec4<f32>( nodeVar243.x ) ) ) * vec4<f32>( nodeVar242.y ) ) + ( ( ( nodeVar248 * vec4<f32>( nodeVar242.x ) ) + ( nodeVar249 * vec4<f32>( nodeVar243.x ) ) ) * vec4<f32>( nodeVar243.y ) ) );

		if ( ( nodeVar175 > 0.0001 ) ) {

			nodeVar251 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar208 + nodeVar223 ), nodeVar220 );
			nodeVar252 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar208 - nodeVar223 ), nodeVar220 );
			nodeVar250 = ( ( ( nodeVar250 + nodeVar251 ) + nodeVar252 ) / vec4<f32>( 3.0 ) );
			

		}

		nodeVar253 = ( ( ( nodeVar202 + nodeVar222 ) * nodeVar221 ) + vec2<f32>( 0.5 ) );
		nodeVar254 = floor( nodeVar253 );
		nodeVar255 = fract( nodeVar253 );
		nodeVar256 = ( ( ( ( nodeVar255 * ( ( nodeVar255 * ( ( - nodeVar255 ) + vec2<f32>( 3.0 ) ) ) - vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( ( nodeVar255 * nodeVar255 ) * ( ( nodeVar255 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) );
		nodeVar257 = ( ( ( ( nodeVar255 * ( ( nodeVar255 * ( ( nodeVar255 * vec2<f32>( -3.0 ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( nodeVar255 * nodeVar255 ) * nodeVar255 ) / vec2<f32>( 6.0 ) ) );
		nodeVar258 = ( ( ( ( ( ( nodeVar255 * nodeVar255 ) * ( ( nodeVar255 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) / nodeVar256 ) - vec2<f32>( 1.0 ) );
		nodeVar259 = ( ( ( ( ( nodeVar255 * nodeVar255 ) * nodeVar255 ) / vec2<f32>( 6.0 ) ) / nodeVar257 ) + vec2<f32>( 1.0 ) );
		nodeVar260 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar254 + vec2<f32>( nodeVar258.x, nodeVar258.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar261 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar254 + vec2<f32>( nodeVar259.x, nodeVar258.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar262 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar254 + vec2<f32>( nodeVar258.x, nodeVar259.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar263 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar254 + vec2<f32>( nodeVar259.x, nodeVar259.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar264 = ( ( ( ( nodeVar260 * vec4<f32>( nodeVar256.x ) ) + ( nodeVar261 * vec4<f32>( nodeVar257.x ) ) ) * vec4<f32>( nodeVar256.y ) ) + ( ( ( nodeVar262 * vec4<f32>( nodeVar256.x ) ) + ( nodeVar263 * vec4<f32>( nodeVar257.x ) ) ) * vec4<f32>( nodeVar257.y ) ) );

		if ( ( nodeVar175 > 0.0001 ) ) {

			nodeVar265 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar202 + nodeVar223 ), nodeVar220 );
			nodeVar266 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar202 - nodeVar223 ), nodeVar220 );
			nodeVar264 = ( ( ( nodeVar264 + nodeVar265 ) + nodeVar266 ) / vec4<f32>( 3.0 ) );
			

		}

		nodeVar267 = ( ( ( nodeVar213 + nodeVar222 ) * nodeVar221 ) + vec2<f32>( 0.5 ) );
		nodeVar268 = floor( nodeVar267 );
		nodeVar269 = fract( nodeVar267 );
		nodeVar270 = ( ( ( ( nodeVar269 * ( ( nodeVar269 * ( ( - nodeVar269 ) + vec2<f32>( 3.0 ) ) ) - vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( ( nodeVar269 * nodeVar269 ) * ( ( nodeVar269 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) );
		nodeVar271 = ( ( ( ( nodeVar269 * ( ( nodeVar269 * ( ( nodeVar269 * vec2<f32>( -3.0 ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 3.0 ) ) ) + vec2<f32>( 1.0 ) ) / vec2<f32>( 6.0 ) ) + ( ( ( nodeVar269 * nodeVar269 ) * nodeVar269 ) / vec2<f32>( 6.0 ) ) );
		nodeVar272 = ( ( ( ( ( ( nodeVar269 * nodeVar269 ) * ( ( nodeVar269 * vec2<f32>( 3.0 ) ) - vec2<f32>( 6.0 ) ) ) + vec2<f32>( 4.0 ) ) / vec2<f32>( 6.0 ) ) / nodeVar270 ) - vec2<f32>( 1.0 ) );
		nodeVar273 = ( ( ( ( ( nodeVar269 * nodeVar269 ) * nodeVar269 ) / vec2<f32>( 6.0 ) ) / nodeVar271 ) + vec2<f32>( 1.0 ) );
		nodeVar274 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar268 + vec2<f32>( nodeVar272.x, nodeVar272.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar275 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar268 + vec2<f32>( nodeVar273.x, nodeVar272.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar276 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar268 + vec2<f32>( nodeVar272.x, nodeVar273.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar277 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( ( ( nodeVar268 + vec2<f32>( nodeVar273.x, nodeVar273.y ) ) - vec2<f32>( 0.5 ) ) / nodeVar221 ), nodeVar220 );
		nodeVar278 = ( ( ( ( nodeVar274 * vec4<f32>( nodeVar270.x ) ) + ( nodeVar275 * vec4<f32>( nodeVar271.x ) ) ) * vec4<f32>( nodeVar270.y ) ) + ( ( ( nodeVar276 * vec4<f32>( nodeVar270.x ) ) + ( nodeVar277 * vec4<f32>( nodeVar271.x ) ) ) * vec4<f32>( nodeVar271.y ) ) );

		if ( ( nodeVar175 > 0.0001 ) ) {

			nodeVar279 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar213 + nodeVar223 ), nodeVar220 );
			nodeVar280 = textureSampleLevel( nodeUniform46, nodeUniform46_sampler, ( nodeVar213 - nodeVar223 ), nodeVar220 );
			nodeVar278 = ( ( ( nodeVar278 + nodeVar279 ) + nodeVar280 ) / vec4<f32>( 3.0 ) );
			

		}

		nodeVar224 = vec4<f32>( nodeVar250.x, nodeVar264.y, nodeVar278.z, nodeVar264.w );
		

	}

	nodeVar281 = nodeVar224.xyz;
	nodeVar281 = ( nodeVar281 + ( ( object.nodeUniform48 * vec3<f32>( clamp( ( 1.0 - nodeVar224.w ), 0.0, 1.0 ) ) ) * vec3<f32>( object.nodeUniform49 ) ) );
	nodeVar282 = object.nodeUniform50;

	if ( ( ( nodeVar282 > 0.0 ) && ( nodeVar282 < 1e30 ) ) ) {

		nodeVar283 = exp( ( - ( ( ( - log( clamp( object.nodeUniform51, vec3<f32>( 0.0001 ), vec3<f32>( 1.0 ) ) ) ) / vec3<f32>( max( nodeVar282, 0.000001 ) ) ) * vec3<f32>( nodeVar203 ) ) ) );

	} else {

		nodeVar283 = vec3<f32>( 1.0, 1.0, 1.0 );

	}

	nodeVar281 = ( nodeVar281 * nodeVar283 );
	nodeVar284 = mix( nodeVar173, ( ( ( nodeVar281 * DiffuseColor.xyz ) * vec3<f32>( ( 1.0 - Metalness ) ) ) * ( vec3<f32>( 1.0 ) - ( SpecularColor + ( ( max( vec3<f32>( ( 1.0 - nodeVar187 ) ), SpecularColor ) - SpecularColor ) * vec3<f32>( pow( ( 1.0 - clamp( dot( nodeVar179, ( - nodeVar176 ) ), 0.0, 1.0 ) ), 5.0 ) ) ) ) ) ), object.nodeUniform52 );
	totalDiffuse = nodeVar284;
	nodeVar285 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar285;
	nodeVar286 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar286;
	nodeVar287 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar287;
	nodeVar288 = normalView;

	// result

	output.color = nodeVar287;

	return output;

}
