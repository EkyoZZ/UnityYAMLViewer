#ifndef COMMON_MATH_INCLUDED
#define COMMON_MATH_INCLUDED

#define PI 3.14159265359
#define DEG2RAD 0.01745329252
#define RAD2DEG 57.29577951

inline half3 BlendOverlay(half3 baseColor, half3 topColor)
{
    return lerp(
        2.0h * baseColor * topColor,
        1.0h - 2.0 * (1.0h - baseColor) * (1.0h - topColor),
        step(0.5h, baseColor)
    );
}

inline half4 ApplyBSC(half4 c, half brightness, half saturation, half contrast)
{
    half3 finalColor = c.rgb * brightness;
    half gray = dot(c.rgb, half3(0.2125h, 0.7154h, 0.0721h));
    finalColor = lerp(half3(gray, gray, gray), finalColor, saturation);
    finalColor = lerp(half3(0.5h, 0.5h, 0.5h), finalColor, contrast);
    return half4(finalColor, c.a);
}

#endif