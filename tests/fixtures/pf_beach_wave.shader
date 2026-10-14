Shader "PFCore/Water/BeachWave"
{
    Properties
    {
        _TintColor      ("Tint Color (RGBA)", Color) = (1,1,1,1)
        _MainTex        ("Foam Color & Area", 2D) = "white" {}
        _Mask           ("Foam Texture", 2D) = "white" {}
        _DistortionTex  ("Distortion Tex", 2D) = "black" {}
        _Distortion_Amount ("Distortion_Amount1", range(0,1) ) = 0
        _R_Uspeed       ("R Channel Uspeed", Float ) = 0
        _R_Vspeed       ("R Channel Vspeed", Float ) = 0
        _AlphaDelay     ("Alpha Delay", Range(-1,1)) = 0
        _Speed          ("Time Scale", Range(0,1)) = 0.25
        _WaveRange      ("Wave Range", Range(-1,1)) = 0.6
        _Layer1OffsetX  ("Layer1 Offset X", Range(-2,2)) = 0
        _Layer2OffsetX  ("Layer2 Offset X", Range(-2,2)) = 0
        _Layer3OffsetX  ("Layer3 Offset X", Range(-2,2)) = 0
        _Layer1OffsetY  ("Layer1 Offset Y", Range(-2,2)) = 0
        _Layer2OffsetY  ("Layer2 Offset Y", Range(-2,2)) = 0
        _Layer3OffsetY  ("Layer3 Offset Y",Range(-2,2)) = 0
        [Toggle] _UseGlobalColor("Use Global Color",Float) = 1
    }
    SubShader
    {
        Tags
        {
            "Queue" = "Transparent"
            "IgnoreProjector" = "True"
            "RenderType" = "Transparent"
            "PreviewType"="Plane"
            "CanUseSpriteAtlas"="True"
        }

        Cull Off
		Lighting Off
        ZWrite off
        Blend srcAlpha OneMinusSrcAlpha

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"
            #include "Assets/Res/Shaders/DayNightSystemShaderHelper.cginc"
            
            //顶点着色器输入结构体
            struct appdata
            {
                float4 vertex : POSITION;   //POSITION输入语义，Unity将顶点坐标提交于此变量
                float4 uv : TEXCOORD0;      //TEXCOORD0，Unity将模型第一套UV提交于此变量
            };

            //顶点着色器输出结构体 （反正会给片段做色器使用）
            struct v2f
            {
                float4 vertex : SV_POSITION;//输出语义
                float4 mainUV : TEXCOORD0; // xy: 泡沫UV, zw: 原始UV
                float4 maskUV12 : TEXCOORD1;
                float2 maskUV3 : TEXCOORD2;
                half3 disappearAlpha : TEXCOORD3;
            };

            //外部变量声明
            sampler2D   _MainTex;
            float4      _MainTex_ST;
            sampler2D   _Mask;
            float4      _Mask_ST;
            half4       _TintColor;
            float       _AlphaDelay;
            sampler2D   _DistortionTex;
            float4      _DistortionTex_ST;
            float       _Distortion_Amount;
            float       _R_Uspeed;
            float       _R_Vspeed;
            float       _Speed;
            float       _WaveRange;
            float       _Layer1OffsetX;
            float       _Layer2OffsetX;
            float       _Layer3OffsetX;
            float       _Layer1OffsetY;
            float       _Layer2OffsetY;
            float       _Layer3OffsetY;

            //UV动画函数
            float2 DelayOffsetUV(float2 uv, float offset_x, float offset_y)//输入UV 和偏移量（x，y），外部变量控制速度，范围
            {
                float pi = 3.1415926536f;
                float sintime = sin(_Time.y * _Speed * pi + offset_y * 0.5f * pi);//余弦函数使UV来回移动,
                float u = offset_x;
                float v = (sintime + 1) * 0.5f * _WaveRange + (1 - _WaveRange);
                uv.x += u;
                uv.y += v;
                return uv;
            }

            //获取泡沫逐渐出现，向岸边移动，开始折返并逐渐消失的透明度值
            half GetDisappearAlpha(float delay)
            {
                const float PI = 3.1415926536f;
                float t = _Time.y * _Speed * PI + delay * 0.5f * PI + 1.2f * PI;
                half a = (sin(t) + 1.0h) * 0.5h;
                return saturate(a * a);
            }

            //顶点着色器
            v2f vert(appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);

                float2 inuv = v.uv;
                o.mainUV.xy = TRANSFORM_TEX(inuv, _MainTex);
                o.mainUV.zw = inuv;

                //每一层的Mask（透贴）偏移，偏移量与tex一致，并可以用_AlphaDelay微调
                // mask1 uv offset
                float2 uv_mask1 = DelayOffsetUV(inuv, _Layer1OffsetX, _Layer1OffsetY - _AlphaDelay);
                o.maskUV12.xy = TRANSFORM_TEX(uv_mask1, _Mask);

                // mask2 uv offset
                float2 uv_mask2 = DelayOffsetUV(inuv, _Layer2OffsetX, _Layer2OffsetY - _AlphaDelay);
                o.maskUV12.zw = TRANSFORM_TEX(uv_mask2, _Mask);

                // mask3 uv offset
                float2 uv_mask3 = DelayOffsetUV(inuv, _Layer3OffsetX, _Layer3OffsetY - _AlphaDelay);
                o.maskUV3 = TRANSFORM_TEX(uv_mask3, _Mask);

                // 透明度动画仅依赖时间和材质参数，放在顶点阶段避免逐像素计算sin。
                o.disappearAlpha = half3(
                    GetDisappearAlpha(_Layer1OffsetY),
                    GetDisappearAlpha(_Layer2OffsetY),
                    GetDisappearAlpha(_Layer3OffsetY));

                return o;
            }

            //片段着色器
            fixed4 frag(v2f i) : SV_Target //参数为输入结构体，语义就是输出到什么地方
            {
                fixed4 foamColor = tex2D(_MainTex, i.mainUV.xy);

                float2 distortionUV = TRANSFORM_TEX(i.mainUV.zw, _DistortionTex);
                distortionUV += float2(_Time.y * _R_Uspeed, _Time.y * _R_Vspeed);
                half2 distortion = tex2D(_DistortionTex, distortionUV).rg;

                half alpha1 = tex2D(_Mask, lerp(i.maskUV12.xy, distortion, _Distortion_Amount)).r * i.disappearAlpha.x;
                half alpha2 = tex2D(_Mask, lerp(i.maskUV12.zw, distortion, _Distortion_Amount)).r * i.disappearAlpha.y;
                half alpha3 = tex2D(_Mask, lerp(i.maskUV3, distortion, _Distortion_Amount)).r * i.disappearAlpha.z;

                // 三层颜色相同，只需合并覆盖率；同时避免原混色函数在alpha为0时除以0。
                half combinedAlpha = 1.0h - (1.0h - alpha1) * (1.0h - alpha2) * (1.0h - alpha3);
                fixed4 finalColor;
                finalColor.rgb = foamColor.rgb * _TintColor.rgb;
                //变量_UseGlobalColor控制是否使用全局颜色
                finalColor.rgb = FinalGlobalColor(finalColor, _GlobalDayNightColor, _ToneMappingMin, _ToneMappingMax, _UseGlobalColor).rgb;
                finalColor.a = combinedAlpha * foamColor.a * _TintColor.a;
                return finalColor;
            }
            ENDCG
        }
    }
}
