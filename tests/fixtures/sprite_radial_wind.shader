Shader "PFW/Sprite/RadialWind"
{
    Properties
    {
        [PerRendererData] _MainTex ("Sprite Texture", 2D) = "white" {}
        _Color ("Tint", Color) = (1,1,1,1)

        [Header(Dont Modify)]
        _OverlayColor ("Overlay Color(叠加颜色)", Color) = (0.5,0.5,0.5,0)
        _AlphaScale("Alpha Scale", Range(0, 1)) = 1
        _Brightness("Brightness(亮度)", Float) = 1
        _Saturation("Saturation(饱和度)", Float) = 1
        _Contrast("Contrast(对比度)", Float) = 1

        [Header(Wind)]
        _WindDirection ("Wind Direction XY", Vector) = (1, 0, 0, 0)
        _WindStrength ("Wind Strength Ratio", Range(0, 0.08)) = 0.025
        _WindSpeed ("Wind Speed", Range(0, 10)) = 2.0
        _WindFrequency ("Wind Frequency", Range(0, 20)) = 3.0
        _GustStrength ("Gust Strength", Range(0, 1)) = 0.35
        _RadialStrength ("Radial Strength Ratio", Range(0, 0.04)) = 0.008
        _PhaseOffset ("Phase Offset", Float) = 0

        [Header(Radial Mask)]
        _Radius ("Effect Radius", Range(0.01, 500)) = 1
        _FixedRadius ("Fixed Center Radius Ratio", Range(0, 0.95)) = 0.16
        _MaskPower ("Mask Power", Range(0.25, 4)) = 2.2

        [HideInInspector] _SpriteRectUV("Sprite Rect UV", Vector) = (0,0,0,0)
        [HideInInspector] _RendererColor ("RendererColor", Color) = (1,1,1,1)
        [HideInInspector] _Flip ("Flip", Vector) = (1,1,1,1)
    }

    SubShader
    {
        Tags
        {
            "Queue"="Transparent"
            "IgnoreProjector"="True"
            "RenderType"="Transparent"
            "PreviewType"="Plane"
            "CanUseSpriteAtlas"="True"
        }

        Cull Off
        Lighting Off
        ZWrite Off
        Blend One OneMinusSrcAlpha

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 2.0
            #pragma multi_compile_instancing

            #include "UnityCG.cginc"
            #include "CGIncludes/CommonMath.cginc"

            sampler2D _MainTex;
            float4 _MainTex_TexelSize;
            fixed4 _Color;
            fixed4 _OverlayColor;
            fixed _AlphaScale;
            half _Brightness;
            half _Saturation;
            half _Contrast;
            fixed4 _RendererColor;
            float4 _Flip;
            float4 _SpriteRectUV;
            float4 _WindDirection;
            float _WindStrength;
            float _WindSpeed;
            float _WindFrequency;
            float _GustStrength;
            float _RadialStrength;
            float _PhaseOffset;
            float _Radius;
            float _FixedRadius;
            float _MaskPower;

            struct appdata_t
            {
                float4 vertex   : POSITION;
                fixed4 color    : COLOR;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_INPUT_INSTANCE_ID
            };

            struct v2f
            {
                float4 vertex   : SV_POSITION;
                fixed4 color    : COLOR;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_OUTPUT_STEREO
            };

            float2 SafeNormalize(float2 value, float2 fallback)
            {
                float lenSq = dot(value, value);
                return lenSq > 0.000001 ? value * rsqrt(lenSq) : fallback;
            }

            float GetSpriteRectUV(float2 uv, out float2 rectUV, out float2 rectSize)
            {
                float hasRectUV = step(0.0001, min(_SpriteRectUV.z, _SpriteRectUV.w));
                float2 textureSize = max(_MainTex_TexelSize.zw, float2(1.0, 1.0));
                float2 rectUVSize = _SpriteRectUV.zw / textureSize;
                rectUV = saturate((uv - _SpriteRectUV.xy) / max(rectUVSize, float2(0.0001, 0.0001)));
                rectUV = lerp(uv, rectUV, hasRectUV);
                rectSize = lerp(textureSize, _SpriteRectUV.zw, hasRectUV);
                return hasRectUV;
            }

            v2f vert(appdata_t v)
            {
                v2f o;
                UNITY_SETUP_INSTANCE_ID(v);
                UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(o);

                float4 local = v.vertex;
                local.xy *= _Flip.xy;

                float2 rectUV;
                float2 rectSize;
                float hasRectUV = GetSpriteRectUV(v.texcoord, rectUV, rectSize);
                float2 rectAspect = rectSize / max(max(rectSize.x, rectSize.y), 1.0);
                float2 rectFromCenter = (rectUV - 0.5) * 2.0 * rectAspect * max(_Radius, 0.0001);
                rectFromCenter *= _Flip.xy;

                float2 fromCenter = lerp(local.xy, rectFromCenter, hasRectUV);
                float distanceFromCenter = length(fromCenter);
                float2 radialDir = SafeNormalize(fromCenter, float2(0, 1));
                float2 tangentDir = float2(-radialDir.y, radialDir.x);
                float2 windDir = _WindDirection.xy;

                float radius = max(_Radius, 0.0001);
                float fixedRadius = saturate(_FixedRadius) * radius;
                float mask = saturate((distanceFromCenter - fixedRadius) / max(radius - fixedRadius, 0.0001));
                float mask2 = mask * mask;
                float mask4 = mask2 * mask2;
                mask = lerp(mask, mask2, saturate(_MaskPower - 1.0));
                mask = lerp(mask, mask4, saturate((_MaskPower - 2.0) * 0.5));

                float phase = _PhaseOffset;
                float baseWave = sin(_Time.y * _WindSpeed + dot(fromCenter, windDir) * _WindFrequency + phase);
                float detailPhase = distanceFromCenter * (_WindFrequency * 0.67) +
                                    dot(radialDir, float2(2.1, 4.3));
                float detailWave = sin(_Time.y * (_WindSpeed * 1.73) + detailPhase + phase * 1.37);
                float windWave = (baseWave + detailWave * 0.65 * _GustStrength) /
                                 (1.0 + 0.65 * _GustStrength);
                float radialWave = detailWave;

                float windFacing = dot(radialDir, windDir) * 0.5 + 0.5;
                float tangentSign = dot(tangentDir, windDir) >= 0 ? 1.0 : -1.0;

                local.xy += windDir * (windWave * _WindStrength * radius * mask);
                local.xy += tangentDir * (windWave * _WindStrength * radius * 0.35 * mask * tangentSign);
                local.xy += radialDir * (radialWave * _RadialStrength * radius * mask * windFacing);

                o.vertex = UnityObjectToClipPos(local);
                o.texcoord = v.texcoord;
                o.color = v.color * _Color * _RendererColor;

                return o;
            }

            fixed4 frag(v2f i) : SV_Target
            {
                fixed4 c = tex2D(_MainTex, i.texcoord) * i.color;
                c = ApplyBSC(c, _Brightness, _Saturation, _Contrast);
                c.rgb = BlendOverlay(c.rgb, _OverlayColor.rgb);
                c.a *= _AlphaScale;
                c.rgb *= c.a;
                return c;
            }
            ENDCG
        }
    }

    FallBack "Sprites/Default"
}
