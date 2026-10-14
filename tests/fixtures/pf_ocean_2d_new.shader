Shader "PFCore/Water/Ocean2D New"
{
    Properties
    {
        [NoScaleOffset]_NormalTex ("Normal Texture", 2D) = "bump" {}
        [NoScaleOffset]_MaskTex   ("Mask (R=Depth, G=Foam, B=Beach)", 2D) = "black" {}
        [NoScaleOffset]_FoamTex   ("Foam Noise", 2D) = "white" {}

        [Header(Water Color)]
        _BeachColor ("Beach Color", Color) = (0.88, 0.82, 0.6, 1)
        _ShoreColor ("Shore Color", Color) = (0.3, 0.73, 0.63, 1)
        _DeepColor  ("Deep Color",  Color) = (0.0, 0.35, 0.55, 1)
        _MaskPower  ("Depth Power", Range(0.5,4)) = 1

        [Header(Water Normal Flow)]
        _Scale ("Normal World Scale", Range(0.01,5)) = 1
        _BumpStrength ("Normal Strength", Range(0,2)) = 1
        _NormalSpeed ("Normal Flow Speed", Range(0,2)) = 0.4
        _BumpTiling    ("Normal Tiling (xy / zw)", Vector) = (1,1, 1.7,1.7)
        _BumpDirection ("Normal Direction (xy / zw)", Vector) = (1,0, -0.6,0.4)

        [Header(Foam Flow)]
        _FoamColor ("Foam Color", Color) = (1,1,1,1)
        _FoamScale ("Foam World Scale", Range(0.01,5)) = 1
        _FoamSpeed ("Foam Flow Speed", Range(0,3)) = 1
        _FoamStrength ("Foam Strength", Range(0,2)) = 1
        _FoamThreshold ("Foam Threshold", Range(0,1)) = 0.4
        _FoamInfluence ("Foam Influence", Range(0,1)) = 0.7
        _FoamTiling    ("Foam Tiling (xy / zw)", Vector) = (1,1, 1.7,1.7)
        _FoamDirection ("Foam Direction (xy / zw)", Vector) = (1,0, -0.6,0.4)

        [Header(Lighting)]
        _SpecPower    ("Specular Power", Range(1,32)) = 8
        _SpecStrength ("Specular Strength", Range(0,10)) = 0.4
        _Brightness ("Brightness", Range(0,2)) = 1
        [Toggle] _UseGlobalColor("Use Global Color", Float) = 1
    }

    SubShader
    {
        Tags { "Queue"="Transparent" "RenderType"="Transparent" "IgnoreProjector"="True" "PreviewType"="Plane" }
        Blend SrcAlpha OneMinusSrcAlpha
        Cull Back
        ZWrite Off
        ZTest LEqual

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"
            #include "Assets/Res/Shaders/DayNightSystemShaderHelper.cginc"

            sampler2D _NormalTex;
            sampler2D _MaskTex;
            sampler2D _FoamTex;

            fixed4 _BeachColor;
            fixed4 _ShoreColor;
            fixed4 _DeepColor;
            fixed4 _FoamColor;

            float _MaskPower;

            float _Scale;
            float _BumpStrength;
            float _NormalSpeed;
            float4 _BumpTiling;
            float4 _BumpDirection;

            float _FoamScale;
            float _FoamSpeed;
            float _FoamStrength;
            float _FoamThreshold;
            float _FoamInfluence;
            float4 _FoamTiling;
            float4 _FoamDirection;

            float _SpecPower;
            float _SpecStrength;
            float _Brightness;

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv     : TEXCOORD0;
            };

            struct v2f
            {
                float4 pos        : SV_POSITION;
                float2 uvMask     : TEXCOORD0;
                float4 bumpCoords : TEXCOORD1;
                float4 foamCoords : TEXCOORD2;
            };

            // 双层法线叠加
            half3 PerPixelNormal(sampler2D bumpMap, float4 coords, half bumpStrength)
            {
                half2 n1 = UnpackNormal(tex2D(bumpMap, coords.xy)).xy;
                half2 n2 = UnpackNormal(tex2D(bumpMap, coords.zw)).xy;
                half2 bump = n1 + n2;

                half3 n;
                n.xy = bump * bumpStrength;
                n.z  = 1;
                return n;
            }

            v2f vert (appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uvMask = v.uv;

                float3 worldPos = mul(unity_ObjectToWorld, v.vertex).xyz;
                worldPos *= 0.08;   // 你之前使用的世界比例

                float4 worldUV = worldPos.xyxy;

                // Normal 流动
                float4 normalFlow = _Time.yyyy * _BumpDirection * _NormalSpeed;
                o.bumpCoords = (worldUV + normalFlow) * _BumpTiling * _Scale;

                // Foam 流动（同样双层方向）
                float4 foamFlow = _Time.yyyy * _FoamDirection * _FoamSpeed;
                o.foamCoords = (worldUV + foamFlow) * _FoamTiling * _FoamScale;

                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                // ================= 水深颜色 =================
                // Mask 的 RGB 使用同一组 UV，合并为一次采样。
                half3 mask = tex2D(_MaskTex, i.uvMask).rgb;
                half depth = mask.r;
                half foamMask = mask.g;
                half beach = mask.b;
                depth = saturate(pow(depth, _MaskPower));
                fixed4 _ShoreMix = lerp(_ShoreColor, _BeachColor, beach);
                fixed4 col = lerp(_DeepColor, _ShoreMix, depth);

                // ================= 水面法线 + 高光 =================
                half3 normal = normalize(PerPixelNormal(_NormalTex, i.bumpCoords, _BumpStrength));
                // 原始光照方向是常量，直接使用预归一化值，避免每像素 normalize。
                const half3 lightDir = half3(0.402h, 0.805h, 0.442h);
                half spec = pow(saturate(dot(normal, lightDir)), _SpecPower) * _SpecStrength;

                // ================= 泡沫 =================
                half foam = 0;
                // 深海区域的 foamMask 为 0，跳过两次泡沫纹理采样。
                UNITY_BRANCH
                if (foamMask > 0.001h)
                {
                    half localThreshold = lerp(
                        _FoamThreshold,
                        _FoamThreshold * (1.0h - _FoamInfluence),
                        foamMask);

                    // 双层泡沫噪声
                    half f1 = tex2D(_FoamTex, i.foamCoords.xy).r;
                    half f2 = tex2D(_FoamTex, i.foamCoords.zw).r;
                    half foamNoise = (f1 + f2) * 0.5h;

                    foam = saturate((foamNoise - localThreshold) * 5.0h);
                    foam *= foamMask * _FoamStrength;
                }

                // ================= 合成 =================
                col.rgb += spec;
                col.rgb = lerp(col.rgb, _FoamColor.rgb, foam);
                col.rgb = FinalGlobalColor(col, _GlobalDayNightWaterColor, _ToneMappingMin, _ToneMappingMax, _UseGlobalColor).rgb;
                col.rgb *= _Brightness;
                return col;
            }
            ENDCG
        }
    }
}
