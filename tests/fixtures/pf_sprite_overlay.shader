// Unity built-in shader source. Copyright (c) 2016 Unity Technologies. MIT license (see license.txt)

Shader "PFCore/SpriteOverlay" {
    Properties 
    {
        _MainTex ("Particle Texture", 2D) = "white" {}
        _AlphaScale("Alpha Scale", Range(0, 1)) = 1
        [Toggle] _Togglelit("Enable Lit", float) = 0
        [Toggle] _ToggleDayLit("Is Day Lit", float) = 0
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
        Blend DstColor SrcAlpha
        ColorMask RGB
        Cull Off Lighting Off ZWrite Off
        Pass 
        {
        CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 2.0

            #include "UnityCG.cginc"

            sampler2D _MainTex;
            fixed _AlphaScale;
            float _Togglelit;
            float _ToggleDayLit;
            fixed _LightIntensity;

            struct appdata_t {
                float4 vertex : POSITION;
                fixed4 color : COLOR;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_INPUT_INSTANCE_ID
            };

            struct v2f {
                float4 vertex : SV_POSITION;
                fixed4 color : COLOR;
                float2 texcoord : TEXCOORD0;
                UNITY_VERTEX_OUTPUT_STEREO
            };

            float4 _MainTex_ST;

            v2f vert (appdata_t v)
            {
                v2f o;
                UNITY_SETUP_INSTANCE_ID(v);
                UNITY_INITIALIZE_VERTEX_OUTPUT_STEREO(o);
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.color = v.color;
                o.texcoord = TRANSFORM_TEX(v.texcoord,_MainTex);
                return o;
            }

            fixed4 frag (v2f i) : SV_Target
            {
                fixed4 col = tex2D(_MainTex, i.texcoord);
                col.a *= lerp(1,abs(_ToggleDayLit - _LightIntensity),_Togglelit) * _AlphaScale;
                col = lerp(col,col*i.color,col.a);
                //col = lerp(col,2*col*i.color*step(col.a,1)+(1-step(col.a,1))*(1-2*(1-col)*(1-i.color)),col.a);
                return col;
            }
        ENDCG
        }
    }
}
