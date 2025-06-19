struct TimingInfo { float currentTimeMs; float lastFrameTimeMs; };
layout(set = 0, binding = 0) uniform TimingBuffer { TimingInfo info; } KOJI_time;
