fixed4 _GlobalDayNightColor;
fixed4 _GlobalDayNightFogColor;
fixed4 _GlobalDayNightWaterColor;
fixed _ToneMappingMin;
fixed _ToneMappingMax;
fixed _UseGlobalColor;

//artist adjusts tone mapping
inline fixed4 LinearToneMapping(fixed4 color, fixed min, fixed max)
{
	fixed luminance = Luminance(color);
	fixed newLuminance = lerp(min, max, luminance);
	return color*newLuminance/(luminance + 0.01);
}
inline fixed4 FinalGlobalColor(fixed4 color, fixed4 globalColor, fixed min, fixed max, fixed global)
{
    fixed4 tempColor = color;
    //tempColor.rgb = LinearToneMapping(tempColor, min, max).rgb;
    tempColor.rgb *= globalColor.rgb;
    return lerp(color, tempColor, global);
}