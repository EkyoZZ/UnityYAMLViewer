Shader "PFCore/Water/RiverCaustics2D"
{
    Properties
    {
        [NoScaleOffset]_CausticsTex ("Caustics Texture", 2D) = "black" {}
        [NoScaleOffset]_MaskTex ("Mask Texture", 2D) = "white" {}

        [Header(Caustics Flow)]
        _Scale ("Caustics World Scale", Range(0.01, 5)) = 0.025
        _CausticsStrength ("Caustics Strength", Range(0, 2)) = 0.35
        _FlowSpeed ("Caustics Flow Speed", Range(0, 2)) = 0.2
        _Tiling ("Caustics Tiling (xy / zw)", Vector) = (1, 1, 1.37, 1.37)
        _Direction ("Caustics Direction (xy / zw)", Vector) = (0.8, 0.35, -0.45, 0.8)

        [Header(Appearance)]
        [HDR]_CausticsColor ("Caustics Color", Color) = (0.55, 0.9, 1, 1)
        _Contrast ("Contrast", Range(0.25, 4)) = 1.5
        _LayerBlend ("Second Layer Blend", Range(0, 1)) = 0.5
        _RGBSplit ("RGB Split", Range(0, 0.05)) = 0.006
        _RGBSplitDirection ("RGB Split Direction", Vector) = (1, 0.35, 0, 0)
    }

    SubShader
    {
        Tags
        {
            "Queue"="Transparent+10"
            "RenderType"="Transparent"
            "IgnoreProjector"="True"
            "PreviewType"="Plane"
        }

        // Add light over the existing water without requiring a main texture.
        Blend SrcAlpha One
        Cull Back
        ZWrite Off
        ZTest LEqual

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _CausticsTex;
            sampler2D _MaskTex;

            float _Scale;
            float _FlowSpeed;
            float4 _Tiling;
            float4 _Direction;

            half4 _CausticsColor;
            half _CausticsStrength;
            half _Contrast;
            half _LayerBlend;
            half _RGBSplit;
            half4 _RGBSplitDirection;

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float4 causticsCoords : TEXCOORD0;
                float2 uvMask : TEXCOORD1;
                float2 rgbOffset : TEXCOORD2;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uvMask = v.uv;

                float3 worldPos = mul(unity_ObjectToWorld, v.vertex).xyz;
                worldPos *= 0.08;

                float4 worldUV = worldPos.xyxy;
                float4 flow = _Time.yyyy * _Direction * _FlowSpeed;
                o.causticsCoords = (worldUV + flow) * _Tiling * _Scale;
                o.rgbOffset = normalize(_RGBSplitDirection.xy + float2(0.0001, 0)) * _RGBSplit;
                return o;
            }

            half3 SampleRGBCaustics(float2 uv, float2 offset)
            {
                half red = tex2D(_CausticsTex, uv + offset).r;
                half green = tex2D(_CausticsTex, uv).r;
                half blue = tex2D(_CausticsTex, uv - offset).r;
                return half3(red, green, blue);
            }

            fixed4 frag(v2f i) : SV_Target
            {
                half mask = tex2D(_MaskTex, i.uvMask).r;
                // 焦散区域外不再执行 6 次纹理采样和后续计算。
                UNITY_BRANCH
                if (mask <= 0.0h)
                {
                    return 0;
                }

                half3 layer1 = SampleRGBCaustics(i.causticsCoords.xy, i.rgbOffset);
                half3 layer2 = SampleRGBCaustics(i.causticsCoords.zw, i.rgbOffset);
                half3 caustics = lerp(layer1, layer1 * layer2, _LayerBlend);
                caustics = pow(saturate(caustics), _Contrast) * _CausticsStrength;

                half alpha = max(caustics.r, max(caustics.g, caustics.b));
                // Normalize hue so SrcAlpha One does not square the intensity.
                half3 color = caustics / max(alpha, 0.0001h) * _CausticsColor.rgb;

                // Alpha controls how much colored light is added to the water below.
                return half4(color, saturate(alpha * _CausticsColor.a * mask));
            }
            ENDCG
        }
    }
}
