
Texture2D texSource : register(t0);
SamplerState sampLinear : register(s0);

cbuffer FxaaConstants : register(b0) {
    float2 rcpFrame;
    float subpixelQuality;
    float edgeThreshold;
    float edgeThresholdMin;
    float qualityPreset;
    float dummy1;
    float dummy2;
};

float rgb2luma(float3 rgb) {
    return dot(rgb, float3(0.299, 0.587, 0.114));
}

struct VSOutput {
    float4 pos : SV_Position;
    float2 uv  : TEXCOORD0;
};

float4 PSMain(VSOutput input) : SV_Target {
    float2 uv = input.uv;
    float4 colorCenter = texSource.SampleLevel(sampLinear, uv, 0);
    float lumaCenter = rgb2luma(colorCenter.rgb);
    
    float lumaDown  = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2(0.0,  rcpFrame.y), 0).rgb);
    float lumaUp    = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2(0.0, -rcpFrame.y), 0).rgb);
    float lumaLeft  = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2(-rcpFrame.x, 0.0), 0).rgb);
    float lumaRight = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2( rcpFrame.x, 0.0), 0).rgb);
    
    float lumaMin = min(lumaCenter, min(min(lumaDown, lumaUp), min(lumaLeft, lumaRight)));
    float lumaMax = max(lumaCenter, max(max(lumaDown, lumaUp), max(lumaLeft, lumaRight)));
    float lumaRange = lumaMax - lumaMin;
    
    if (lumaRange < max(edgeThresholdMin, lumaMax * edgeThreshold)) {
        return colorCenter;
    }
    
    float lumaDownLeft  = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2(-rcpFrame.x,  rcpFrame.y), 0).rgb);
    float lumaUpRight    = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2( rcpFrame.x, -rcpFrame.y), 0).rgb);
    float lumaUpLeft    = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2(-rcpFrame.x, -rcpFrame.y), 0).rgb);
    float lumaDownRight = rgb2luma(texSource.SampleLevel(sampLinear, uv + float2( rcpFrame.x,  rcpFrame.y), 0).rgb);
    
    float lumaDownUp = lumaDown + lumaUp;
    float lumaLeftRight = lumaLeft + lumaRight;
    float lumaLeftCorners = lumaDownLeft + lumaUpLeft;
    float lumaDownCorners = lumaDownLeft + lumaDownRight;
    float lumaRightCorners = lumaDownRight + lumaUpRight;
    float lumaUpCorners = lumaUpRight + lumaUpLeft;
    
    float edgeHorizontal = abs(-2.0 * lumaLeft + lumaLeftCorners) +
                           abs(-2.0 * lumaCenter + lumaDownUp) * 2.0 +
                           abs(-2.0 * lumaRight + lumaRightCorners);
                           
    float edgeVertical = abs(-2.0 * lumaUp + lumaUpCorners) +
                         abs(-2.0 * lumaCenter + lumaLeftRight) * 2.0 +
                         abs(-2.0 * lumaDown + lumaDownCorners);
                         
    bool isHorizontal = (edgeHorizontal >= edgeVertical);
    
    float luma1 = isHorizontal ? lumaDown : lumaLeft;
    float luma2 = isHorizontal ? lumaUp : lumaRight;
    float gradient1 = luma1 - lumaCenter;
    float gradient2 = luma2 - lumaCenter;
    
    bool is1Steeper = abs(gradient1) >= abs(gradient2);
    float gradientScaled = 0.25 * max(abs(gradient1), abs(gradient2));
    
    float stepLength = isHorizontal ? rcpFrame.y : rcpFrame.x;
    float lumaLocalAverage = 0.0;
    
    if (is1Steeper) {
        stepLength = -stepLength;
        lumaLocalAverage = 0.5 * (luma1 + lumaCenter);
    } else {
        lumaLocalAverage = 0.5 * (luma2 + lumaCenter);
    }
    
    float2 currentUv = uv;
    if (isHorizontal) {
        currentUv.y += stepLength * 0.5;
    } else {
        currentUv.x += stepLength * 0.5;
    }
    
    float2 offset = isHorizontal ? float2(rcpFrame.x, 0.0) : float2(0.0, rcpFrame.y);
    float2 uvPos = currentUv + offset;
    float2 uvNeg = currentUv - offset;
    
    float lumaEndPos = rgb2luma(texSource.SampleLevel(sampLinear, uvPos, 0).rgb) - lumaLocalAverage;
    float lumaEndNeg = rgb2luma(texSource.SampleLevel(sampLinear, uvNeg, 0).rgb) - lumaLocalAverage;
    
    bool reachedPos = abs(lumaEndPos) >= gradientScaled;
    bool reachedNeg = abs(lumaEndNeg) >= gradientScaled;
    
    if (!reachedPos) uvPos += offset;
    if (!reachedNeg) uvNeg -= offset;
    
    static const int MAX_STEPS = 10;
    static const float QUALITY[10] = { 1.0, 1.0, 1.0, 1.0, 1.5, 2.0, 2.0, 2.0, 4.0, 8.0 };
    
    [unroll(10)]
    for (int i = 2; i < MAX_STEPS; ++i) {
        if (!reachedPos) {
            lumaEndPos = rgb2luma(texSource.SampleLevel(sampLinear, uvPos, 0).rgb) - lumaLocalAverage;
            reachedPos = abs(lumaEndPos) >= gradientScaled;
            if (!reachedPos) uvPos += offset * QUALITY[i];
        }
        if (!reachedNeg) {
            lumaEndNeg = rgb2luma(texSource.SampleLevel(sampLinear, uvNeg, 0).rgb) - lumaLocalAverage;
            reachedNeg = abs(lumaEndNeg) >= gradientScaled;
            if (!reachedNeg) uvNeg -= offset * QUALITY[i];
        }
        if (reachedPos && reachedNeg) break;
    }
    
    float distPos = isHorizontal ? (uvPos.x - uv.x) : (uvPos.y - uv.y);
    float distNeg = isHorizontal ? (uv.x - uvNeg.x) : (uv.y - uvNeg.y);
    
    bool isDirectionPos = distPos < distNeg;
    float distFinal = min(distPos, distNeg);
    float edgeThickness = distPos + distNeg;
    
    float lumaAverageCorners = 2.0 * (lumaDownUp + lumaLeftRight) + (lumaLeftCorners + lumaRightCorners);
    float subPixelOffset1 = clamp(abs((lumaAverageCorners / 12.0) - lumaCenter) / lumaRange, 0.0, 1.0);
    float subPixelOffset2 = (-2.0 * subPixelOffset1 + 3.0) * subPixelOffset1 * subPixelOffset1;
    float subPixelOffsetFinal = subPixelOffset2 * subPixelOffset2 * subpixelQuality;
    
    float pixelOffset = -distFinal / edgeThickness + 0.5;
    bool isLumaCenterSmaller = lumaCenter < lumaLocalAverage;
    bool correctVariation = ((isDirectionPos ? lumaEndPos : lumaEndNeg) < 0.0) != isLumaCenterSmaller;
    float finalOffset = correctVariation ? pixelOffset : 0.0;
    
    finalOffset = max(finalOffset, subPixelOffsetFinal);
    
    float2 finalUv = uv;
    if (isHorizontal) {
        finalUv.y += finalOffset * stepLength;
    } else {
        finalUv.x += finalOffset * stepLength;
    }
    
    return texSource.SampleLevel(sampLinear, finalUv, 0);
}
