[numthreads(8, 8, 1)]
void main(uint3 dispatchThreadId : SV_DispatchThreadID)
{
    RWTexture2D<float4> output_image = ResourceDescriptorHeap[0];
    bool is_white = ((dispatchThreadId.x + dispatchThreadId.y) & 1) == 0;
    float channel = is_white ? 1.0f : 0.0f;
    output_image[dispatchThreadId.xy] = float4(channel, channel, channel, 1.0f);
}
