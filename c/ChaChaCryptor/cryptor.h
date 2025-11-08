#include "CommonServices.h"

OSStatus chachadecrypt_64x64(uint8_t key[32], uint8_t nonce[8], void *inHeader, void *inFramePtr, uint32_t bodySize, void *inDst);