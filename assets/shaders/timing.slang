struct TimingInfo { float currentTimeMs; float lastFrameTimeMs; };

#if defined(__HLSL__) || defined(__SLANG__)
struct TimingBuffer { TimingInfo info; };
[[vk::binding(0, 0)]] ConstantBuffer<TimingBuffer> KOJI_time;
#else
layout(set = 0, binding = 0) uniform TimingBuffer { TimingInfo info; } KOJI_time;
#endif
