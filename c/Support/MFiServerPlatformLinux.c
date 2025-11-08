/*
	File:    	MFiServerPlatformLinux.c
	Package: 	Apple CarPlay Communication Plug-in.
	Abstract: 	n/a 
	Version: 	410.12
	
	Disclaimer: IMPORTANT: This Apple software is supplied to you, by Apple Inc. ("Apple"), in your
	capacity as a current, and in good standing, Licensee in the MFi Licensing Program. Use of this
	Apple software is governed by and subject to the terms and conditions of your MFi License,
	including, but not limited to, the restrictions specified in the provision entitled ”Public 
	Software”, and is further subject to your agreement to the following additional terms, and your 
	agreement that the use, installation, modification or redistribution of this Apple software
	constitutes acceptance of these additional terms. If you do not agree with these additional terms,
	please do not use, install, modify or redistribute this Apple software.
	
	Subject to all of these terms and in consideration of your agreement to abide by them, Apple grants
	you, for as long as you are a current and in good-standing MFi Licensee, a personal, non-exclusive 
	license, under Apple's copyrights in this original Apple software (the "Apple Software"), to use, 
	reproduce, and modify the Apple Software in source form, and to use, reproduce, modify, and 
	redistribute the Apple Software, with or without modifications, in binary form. While you may not 
	redistribute the Apple Software in source form, should you redistribute the Apple Software in binary
	form, you must retain this notice and the following text and disclaimers in all such redistributions
	of the Apple Software. Neither the name, trademarks, service marks, or logos of Apple Inc. may be
	used to endorse or promote products derived from the Apple Software without specific prior written
	permission from Apple. Except as expressly stated in this notice, no other rights or licenses, 
	express or implied, are granted by Apple herein, including but not limited to any patent rights that
	may be infringed by your derivative works or by other works in which the Apple Software may be 
	incorporated.  
	
	Unless you explicitly state otherwise, if you provide any ideas, suggestions, recommendations, bug 
	fixes or enhancements to Apple in connection with this software (“Feedback”), you hereby grant to
	Apple a non-exclusive, fully paid-up, perpetual, irrevocable, worldwide license to make, use, 
	reproduce, incorporate, modify, display, perform, sell, make or have made derivative works of,
	distribute (directly or indirectly) and sublicense, such Feedback in connection with Apple products 
	and services. Providing this Feedback is voluntary, but if you do provide Feedback to Apple, you 
	acknowledge and agree that Apple may exercise the license granted above without the payment of 
	royalties or further consideration to Participant.
	
	The Apple Software is provided by Apple on an "AS IS" basis. APPLE MAKES NO WARRANTIES, EXPRESS OR 
	IMPLIED, INCLUDING WITHOUT LIMITATION THE IMPLIED WARRANTIES OF NON-INFRINGEMENT, MERCHANTABILITY 
	AND FITNESS FOR A PARTICULAR PURPOSE, REGARDING THE APPLE SOFTWARE OR ITS USE AND OPERATION ALONE OR
	IN COMBINATION WITH YOUR PRODUCTS.
	
	IN NO EVENT SHALL APPLE BE LIABLE FOR ANY SPECIAL, INDIRECT, INCIDENTAL OR CONSEQUENTIAL DAMAGES 
	(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
	PROFITS; OR BUSINESS INTERRUPTION) ARISING IN ANY WAY OUT OF THE USE, REPRODUCTION, MODIFICATION 
	AND/OR DISTRIBUTION OF THE APPLE SOFTWARE, HOWEVER CAUSED AND WHETHER UNDER THEORY OF CONTRACT, TORT
	(INCLUDING NEGLIGENCE), STRICT LIABILITY OR OTHERWISE, EVEN IF APPLE HAS BEEN ADVISED OF THE 
	POSSIBILITY OF SUCH DAMAGE.
	
	Copyright (C) 2013-2014 Apple Inc. All Rights Reserved. Not to be used or disclosed without permission from Apple.
	
	Linux platform plugin for MFi-SAP authentication/encryption.
	
	This defaults to using i2c bus /dev/i2c-1 with an MFi auth IC device address of 0x11 (RST pulled high).
	These can be overridden in the makefile with the following:
	
	CFLAGS += -DMFI_AUTH_DEVICE_PATH=\"/dev/i2c-0\"		# MFi auth IC on i2c bus /dev/i2c-0.
	CFLAGS += -DMFI_AUTH_DEVICE_ADDRESS=0x10			# MFi auth IC at address 0x10 (RST pulled low).
*/

#include "MFiSAP.h"

#include <fcntl.h>

#if(!TARGET_OS_DARWIN)
    #include <linux/i2c-dev.h>
    #include <linux/i2c.h>
#endif

#include <stdlib.h>
#include <sys/ioctl.h>
#include <sys/types.h>
#include <unistd.h>

#include <openssl/aes.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/rsa.h>


#include "CommonServices.h"
#include "DebugServices.h"
#include "TickUtils.h"

//===========================================================================================================================
//	Constants
//===========================================================================================================================

#if( defined( MFI_AUTH_DEVICE_PATH ) )
	#define kMFiAuthDevicePath					MFI_AUTH_DEVICE_PATH
#else
	#define kMFiAuthDevicePath					"/dev/i2c-1"
#endif

#if( defined( MFI_AUTH_DEVICE_ADDRESS ) )
	#define kMFiAuthDeviceAddress				MFI_AUTH_DEVICE_ADDRESS
#else
	#define kMFiAuthDeviceAddress				0x11
#endif

#define kMFiAuthRetryDelayMics					5000 // 5 ms.

#define kMFiAuthReg_AuthControlStatus			0x10
	#define kMFiAuthFlagError						0x80
	#define kMFiAuthControl_GenerateSignature		1
#define kMFiAuthReg_SignatureSize				0x11
#define kMFiAuthReg_SignatureData				0x12
#define kMFiAuthReg_ChallengeSize				0x20
#define kMFiAuthReg_ChallengeData				0x21
#define kMFiAuthReg_DeviceCertificateSize		0x30
#define kMFiAuthReg_DeviceCertificateData1		0x31 // Note: auto-increments so next read is Data2, Data3, etc.

static const uint8_t kPublicCertificate[] = {
        0x30, 0x82, 0x03, 0xad, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02, 0xa0,
        0x82, 0x03, 0x9e, 0x30, 0x82, 0x03, 0x9a, 0x02, 0x01, 0x01, 0x31, 0x00, 0x30, 0x0b, 0x06, 0x09,
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01, 0xa0, 0x82, 0x03, 0x80, 0x30, 0x82, 0x03,
        0x7c, 0x30, 0x82, 0x02, 0x64, 0xa0, 0x03, 0x02, 0x01, 0x02, 0x02, 0x0f, 0x33, 0x33, 0xaa, 0x12,
        0x12, 0x19, 0xaa, 0x07, 0xaa, 0x00, 0x06, 0xaa, 0x00, 0x00, 0x01, 0x30, 0x0d, 0x06, 0x09, 0x2a,
        0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x05, 0x05, 0x00, 0x30, 0x81, 0x92, 0x31, 0x0b, 0x30,
        0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x31, 0x1d, 0x30, 0x1b, 0x06, 0x03,
        0x55, 0x04, 0x0a, 0x13, 0x14, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x43, 0x6f, 0x6d, 0x70, 0x75,
        0x74, 0x65, 0x72, 0x2c, 0x20, 0x49, 0x6e, 0x63, 0x2e, 0x31, 0x2d, 0x30, 0x2b, 0x06, 0x03, 0x55,
        0x04, 0x0b, 0x13, 0x24, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x43, 0x6f, 0x6d, 0x70, 0x75, 0x74,
        0x65, 0x72, 0x20, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66, 0x69, 0x63, 0x61, 0x74, 0x65, 0x20, 0x41,
        0x75, 0x74, 0x68, 0x6f, 0x72, 0x69, 0x74, 0x79, 0x31, 0x35, 0x30, 0x33, 0x06, 0x03, 0x55, 0x04,
        0x03, 0x13, 0x2c, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x69, 0x50, 0x6f, 0x64, 0x20, 0x41, 0x63,
        0x63, 0x65, 0x73, 0x73, 0x6f, 0x72, 0x69, 0x65, 0x73, 0x20, 0x43, 0x65, 0x72, 0x74, 0x69, 0x66,
        0x69, 0x63, 0x61, 0x74, 0x65, 0x20, 0x41, 0x75, 0x74, 0x68, 0x6f, 0x72, 0x69, 0x74, 0x79, 0x30,
        0x1e, 0x17, 0x0d, 0x31, 0x32, 0x31, 0x32, 0x31, 0x39, 0x32, 0x30, 0x31, 0x34, 0x34, 0x38, 0x5a,
        0x17, 0x0d, 0x32, 0x30, 0x31, 0x32, 0x31, 0x39, 0x32, 0x30, 0x31, 0x34, 0x34, 0x38, 0x5a, 0x30,
        0x81, 0x83, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x06, 0x13, 0x02, 0x55, 0x53, 0x31,
        0x1d, 0x30, 0x1b, 0x06, 0x03, 0x55, 0x04, 0x0a, 0x13, 0x14, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20,
        0x43, 0x6f, 0x6d, 0x70, 0x75, 0x74, 0x65, 0x72, 0x2c, 0x20, 0x49, 0x6e, 0x63, 0x2e, 0x31, 0x28,
        0x30, 0x26, 0x06, 0x03, 0x55, 0x04, 0x0b, 0x13, 0x1f, 0x41, 0x70, 0x70, 0x6c, 0x65, 0x20, 0x43,
        0x6f, 0x6d, 0x70, 0x75, 0x74, 0x65, 0x72, 0x20, 0x69, 0x50, 0x6f, 0x64, 0x20, 0x41, 0x63, 0x63,
        0x65, 0x73, 0x73, 0x6f, 0x72, 0x69, 0x65, 0x73, 0x31, 0x2b, 0x30, 0x29, 0x06, 0x03, 0x55, 0x04,
        0x03, 0x14, 0x22, 0x49, 0x50, 0x41, 0x5f, 0x33, 0x33, 0x33, 0x33, 0x41, 0x41, 0x31, 0x32, 0x31,
        0x32, 0x31, 0x39, 0x41, 0x41, 0x30, 0x37, 0x41, 0x41, 0x30, 0x30, 0x30, 0x36, 0x41, 0x41, 0x30,
        0x30, 0x30, 0x30, 0x30, 0x31, 0x30, 0x81, 0x9f, 0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86,
        0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00, 0x03, 0x81, 0x8d, 0x00, 0x30, 0x81, 0x89, 0x02, 0x81,
        0x81, 0x00, 0xdf, 0x32, 0x0f, 0x50, 0xef, 0xa1, 0x5a, 0xd1, 0x3c, 0xd5, 0x21, 0xda, 0x01, 0x22,
        0x6a, 0xd4, 0xf4, 0x3f, 0x99, 0x85, 0x97, 0x0e, 0x7e, 0x3a, 0xab, 0x91, 0x8e, 0x8e, 0xf3, 0xda,
        0xbf, 0x78, 0xe6, 0xf9, 0xcc, 0x12, 0x26, 0x53, 0xaf, 0x77, 0x9f, 0x32, 0x84, 0xd2, 0x28, 0xb2,
        0x58, 0xca, 0xe2, 0xfb, 0x8e, 0x35, 0x19, 0xe7, 0x15, 0x0d, 0xfe, 0x98, 0xa5, 0x4d, 0x8b, 0xe1,
        0x69, 0x48, 0xbc, 0x8e, 0x85, 0x16, 0x1a, 0x99, 0x89, 0x00, 0xf6, 0x8e, 0xe5, 0x18, 0x28, 0xfd,
        0xe4, 0x9d, 0x26, 0xb3, 0xaf, 0x30, 0xa4, 0x94, 0x9c, 0x49, 0x0c, 0x4f, 0xa4, 0x7c, 0xb6, 0x0e,
        0xbc, 0xa1, 0x66, 0x4f, 0x26, 0x52, 0xb9, 0xd6, 0xb2, 0x22, 0x69, 0x19, 0xa4, 0x53, 0x7d, 0x88,
        0x8c, 0x21, 0x12, 0x58, 0xb1, 0xe1, 0xc2, 0x89, 0x59, 0xd9, 0x36, 0x7e, 0x8b, 0x73, 0x7b, 0x0b,
        0x81, 0xdb, 0x02, 0x03, 0x01, 0x00, 0x01, 0xa3, 0x60, 0x30, 0x5e, 0x30, 0x0e, 0x06, 0x03, 0x55,
        0x1d, 0x0f, 0x01, 0x01, 0xff, 0x04, 0x04, 0x03, 0x02, 0x03, 0xb8, 0x30, 0x0c, 0x06, 0x03, 0x55,
        0x1d, 0x13, 0x01, 0x01, 0xff, 0x04, 0x02, 0x30, 0x00, 0x30, 0x1d, 0x06, 0x03, 0x55, 0x1d, 0x0e,
        0x04, 0x16, 0x04, 0x14, 0xdd, 0x7c, 0x4f, 0xe5, 0xda, 0xe3, 0xe5, 0x95, 0xea, 0x88, 0x51, 0xb9,
        0xf3, 0xcf, 0x1a, 0x56, 0x2c, 0x42, 0x40, 0xab, 0x30, 0x1f, 0x06, 0x03, 0x55, 0x1d, 0x23, 0x04,
        0x18, 0x30, 0x16, 0x80, 0x14, 0xc9, 0xaa, 0x84, 0x6b, 0x06, 0xb8, 0x76, 0xe2, 0x96, 0x4f, 0xe7,
        0x27, 0x02, 0xd7, 0x2e, 0x3b, 0xda, 0xf7, 0xb0, 0x18, 0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48,
        0x86, 0xf7, 0x0d, 0x01, 0x01, 0x05, 0x05, 0x00, 0x03, 0x82, 0x01, 0x01, 0x00, 0x11, 0xcd, 0x90,
        0x1a, 0xcb, 0x45, 0xa2, 0x64, 0xfc, 0x16, 0x87, 0xc3, 0xb2, 0x1c, 0x00, 0xeb, 0x06, 0xae, 0x94,
        0xe4, 0x06, 0x40, 0xf9, 0x4b, 0x49, 0x8a, 0xa0, 0x15, 0xd4, 0xb0, 0xca, 0xff, 0x8f, 0xc9, 0xdd,
        0x52, 0xd5, 0x8e, 0xb7, 0xc4, 0x45, 0x5c, 0xad, 0x54, 0x86, 0x0c, 0x6a, 0x22, 0x08, 0xb2, 0x36,
        0xd3, 0xc6, 0x02, 0xf5, 0xdf, 0x0f, 0xd4, 0xac, 0x39, 0x98, 0x28, 0x18, 0xbc, 0xc7, 0xd7, 0x92,
        0x91, 0x70, 0x2a, 0xf2, 0x86, 0xa1, 0x52, 0x25, 0x6d, 0x6b, 0xab, 0xc3, 0xf3, 0x94, 0x7e, 0x6a,
        0x98, 0x14, 0x35, 0x0c, 0xf9, 0x7c, 0xc0, 0xf2, 0x03, 0x55, 0x74, 0xdf, 0xf1, 0x58, 0x3a, 0x74,
        0x41, 0x24, 0x9e, 0x44, 0xb8, 0xbf, 0x19, 0x7f, 0x4f, 0x45, 0xac, 0x56, 0x42, 0xb0, 0x42, 0xbe,
        0x00, 0xa4, 0x7e, 0x35, 0xff, 0x3c, 0x23, 0x77, 0xec, 0x36, 0xa4, 0xab, 0x7e, 0x54, 0x3f, 0x3f,
        0xee, 0xd0, 0x58, 0x5b, 0x35, 0xcb, 0x36, 0x5c, 0x80, 0x7b, 0xc3, 0xcb, 0x65, 0x71, 0x52, 0x3c,
        0x90, 0x72, 0xc6, 0x72, 0x58, 0xd3, 0x57, 0x6c, 0x33, 0xff, 0x8d, 0xda, 0x4d, 0xb0, 0x4c, 0x88,
        0x9d, 0x91, 0x41, 0x8b, 0xc6, 0xa7, 0x86, 0x63, 0x6b, 0xc2, 0x44, 0xa4, 0x7d, 0x08, 0x8f, 0xc0,
        0xa4, 0x29, 0xd6, 0xdf, 0xe6, 0x9f, 0x3d, 0xec, 0xe9, 0x63, 0x77, 0xc0, 0x64, 0xed, 0x93, 0x07,
        0x92, 0x0a, 0x7e, 0x9c, 0xc3, 0x8b, 0x21, 0x20, 0xf0, 0xf2, 0xbc, 0xa6, 0x68, 0x3d, 0xe7, 0x8b,
        0x0d, 0x10, 0xc2, 0x75, 0x3a, 0x18, 0xe8, 0x04, 0x92, 0xdb, 0xb9, 0x45, 0x85, 0x2a, 0x9a, 0xb7,
        0x9d, 0xf7, 0x53, 0xaf, 0xea, 0xc3, 0x44, 0xea, 0x6a, 0xef, 0xb5, 0xda, 0x4d, 0x2a, 0x11, 0x0d,
        0x84, 0xcb, 0x86, 0x0c, 0xd4, 0xaa, 0xc9, 0x47, 0xab, 0x04, 0xfd, 0x70, 0xac, 0xa1, 0x00, 0x31,
        0x00
};

//===========================================================================================================================
//	Prototypes
//===========================================================================================================================

static OSStatus
	_DoI2C( 
		int				inFD, 
		uint8_t			inRegister, 
		const uint8_t *	inWritePtr, 
		size_t			inWriteLen, 
		uint8_t *		inReadBuf, 
		size_t			inReadLen );

//===========================================================================================================================
//	Globals
//===========================================================================================================================

static uint8_t *		gMFiCertificatePtr	= NULL;
static size_t			gMFiCertificateLen	= 0;

int RSASign(RSA* rsa, const unsigned char* Msg, size_t MsgLen, unsigned char** EncMsg, size_t* MsgLenEnc)
{
    EVP_MD_CTX* m_RSASignCtx = EVP_MD_CTX_create();
    EVP_PKEY* priKey  = EVP_PKEY_new();
    EVP_PKEY_assign_RSA(priKey, rsa);
    if (EVP_DigestSignInit(m_RSASignCtx,NULL, EVP_sha256(), NULL,priKey)<=0) {
        return 0;
    }
    if (EVP_DigestSignUpdate(m_RSASignCtx, Msg, MsgLen) <= 0) {
        return 0;
    }
    if (EVP_DigestSignFinal(m_RSASignCtx, NULL, MsgLenEnc) <=0) {
        return 0;
    }
    *EncMsg = (unsigned char*)malloc(*MsgLenEnc);
    if (EVP_DigestSignFinal(m_RSASignCtx, *EncMsg, MsgLenEnc) <= 0) {
        return 0;
    }
    EVP_MD_CTX_free(m_RSASignCtx);
    return 1;
}

RSA* createPrivateRSA()
{
    RSA *rsa = NULL;
    const char* c_string = "-----BEGIN RSA PRIVATE KEY-----\n"\
                         "MIIEowIBAAKCAQEAy8Dbv8prpJ/0kKhlGeJYozo2t60EG8L0561g13R29LvMR5hy\n"\
                         "vGZlGJpmn65+A4xHXInJYiPuKzrKUnApeLZ+vw1HocOAZtWK0z3r26uA8kQYOKX9\n"\
                         "Qt/DbCdvsF9wF8gRK0ptx9M6R13NvBxvVQApfc9jB9nTzphOgM4JiEYvlV8FLhg9\n"\
                         "yZovMYd6Wwf3aoXK891VQxTr/kQYoq1Yp+68i6T4nNq7NWC+UNVjQHxNQMQMzU6l\n"\
                         "WCX8zyg3yH88OAQkUXIXKfQ+NkvYQ1cxaMoVPpY72+eVthKzpMeyHkBn7ciumk5q\n"\
                         "gLTEJAfWZpe4f4eFZj/Rc8Y8Jj2IS5kVPjUywQIDAQABAoIBADhg1u1Mv1hAAlX8\n"\
                         "omz1Gn2f4AAW2aos2cM5UDCNw1SYmj+9SRIkaxjRsE/C4o9sw1oxrg1/z6kajV0e\n"\
                         "N/t008FdlVKHXAIYWF93JMoVvIpMmT8jft6AN/y3NMpivgt2inmmEJZYNioFJKZG\n"\
                         "X+/vKYvsVISZm2fw8NfnKvAQK55yu+GRWBZGOeS9K+LbYvOwcrjKhHz66m4bedKd\n"\
                         "gVAix6NE5iwmjNXktSQlJMCjbtdNXg/xo1/G4kG2p/MO1HLcKfe1N5FgBiXj3Qjl\n"\
                         "vgvjJZkh1as2KTgaPOBqZaP03738VnYg23ISyvfT/teArVGtxrmFP7939EvJFKpF\n"\
                         "1wTxuDkCgYEA7t0DR37zt+dEJy+5vm7zSmN97VenwQJFWMiulkHGa0yU3lLasxxu\n"\
                         "m0oUtndIjenIvSx6t3Y+agK2F3EPbb0AZ5wZ1p1IXs4vktgeQwSSBdqcM8LZFDvZ\n"\
                         "uPboQnJoRdIkd62XnP5ekIEIBAfOp8v2wFpSfE7nNH2u4CpAXNSF9HsCgYEA2l8D\n"\
                         "JrDE5m9Kkn+J4l+AdGfeBL1igPF3DnuPoV67BpgiaAgI4h25UJzXiDKKoa706S0D\n"\
                         "4XB74zOLX11MaGPMIdhlG+SgeQfNoC5lE4ZWXNyESJH1SVgRGT9nBC2vtL6bxCVV\n"\
                         "WBkTeC5D6c/QXcai6yw6OYyNNdp0uznKURe1xvMCgYBVYYcEjWqMuAvyferFGV+5\n"\
                         "nWqr5gM+yJMFM2bEqupD/HHSLoeiMm2O8KIKvwSeRYzNohKTdZ7FwgZYxr8fGMoG\n"\
                         "PxQ1VK9DxCvZL4tRpVaU5Rmknud9hg9DQG6xIbgIDR+f79sb8QjYWmcFGc1SyWOA\n"\
                         "SkjlykZ2yt4xnqi3BfiD9QKBgGqLgRYXmXp1QoVIBRaWUi55nzHg1XbkWZqPXvz1\n"\
                         "I3uMLv1jLjJlHk3euKqTPmC05HoApKwSHeA0/gOBmg404xyAYJTDcCidTg6hlF96\n"\
                         "ZBja3xApZuxqM62F6dV4FQqzFX0WWhWp5n301N33r0qR6FumMKJzmVJ1TA8tmzEF\n"\
                         "yINRAoGBAJqioYs8rK6eXzA8ywYLjqTLu/yQSLBn/4ta36K8DyCoLNlNxSuox+A5\n"\
                         "w6z2vEfRVQDq4Hm4vBzjdi3QfYLNkTiTqLcvgWZ+eX44ogXtdTDO7c+GeMKWz4XX\n"\
                         "uJSUVL5+CVjKLjZEJ6Qc2WZLl94xSwL71E41H4YciVnSCQxVc4Jw\n"\
                         "-----END RSA PRIVATE KEY-----\n\0";

    BIO * keybio = BIO_new_mem_buf((void*)c_string, -1);
    if (keybio==NULL) {
        return 0;
    }
    rsa = PEM_read_bio_RSAPrivateKey(keybio, &rsa,NULL, NULL);
    return rsa;
}

//===========================================================================================================================
//	MFiPlatform_Initialize
//===========================================================================================================================

OSStatus	MFiPlatform_Initialize( void )
{
	// Cache the certificate at startup since the certificate doesn't change and this saves ~200 ms each time.
	
	MFiPlatform_CopyCertificate( &gMFiCertificatePtr, &gMFiCertificateLen );
	return( kNoErr );
}

//===========================================================================================================================
//	MFiPlatform_Finalize
//===========================================================================================================================

void	MFiPlatform_Finalize( void )
{
	ForgetMem( &gMFiCertificatePtr );
	gMFiCertificateLen = 0;
}

//===========================================================================================================================
//	MFiPlatform_CreateSignature
//===========================================================================================================================

OSStatus
	MFiPlatform_CreateSignature( 
		const void *	inDigestPtr, 
		size_t			inDigestLen, 
		uint8_t **		outSignaturePtr,
		size_t *		outSignatureLen )
{
    OSStatus err;
    uint8_t digestBuf[32];
    const void* digestPtr;
    size_t digestLen;
    size_t skLen;

    SHA1(inDigestPtr, inDigestLen, digestBuf);
    digestLen = SHA_DIGEST_LENGTH;
    digestPtr = digestBuf;

    RSA* privateRSA = createPrivateRSA();
    RSASign(privateRSA, digestPtr, digestLen, outSignaturePtr, outSignatureLen);
    err = kNoErr;

exit:
    return (err);
}

//===========================================================================================================================
//	MFiPlatform_CopyCertificate
//===========================================================================================================================

OSStatus	MFiPlatform_CopyCertificate( uint8_t **outCertificatePtr, size_t *outCertificateLen )
{
    OSStatus err;
    size_t skLen;
    uint8_t* ptr;

    dlog( kLogLevelVerbose, "MFi auth copy certificate\n" );

    if (outCertificatePtr) {
        ptr = (uint8_t*)malloc(sizeof(kPublicCertificate));
        require_action(ptr, exit, err = kNoMemoryErr);
        memcpy(ptr, kPublicCertificate, sizeof(kPublicCertificate));
        *outCertificatePtr = ptr;
    }
    if (outCertificateLen)
        *outCertificateLen = sizeof(kPublicCertificate);
    err = kNoErr;

    dlog( kLogLevelVerbose, "MFi auth copy certificate done: %zu bytes\n", *outCertificateLen );

exit:
    if( err )		dlog( kLogLevelWarning, "### MFi auth copy certificate failed: %#m\n", err );
    return (err);
}

//===========================================================================================================================
//	_DoI2C
//===========================================================================================================================

static OSStatus
	_DoI2C( 
		int				inFD, 
		uint8_t			inRegister, 
		const uint8_t *	inWritePtr, 
		size_t			inWriteLen, 
		uint8_t *		inReadBuf, 
		size_t			inReadLen )
{
	OSStatus		err;
	uint64_t		deadline;
	int				tries;
	ssize_t			n;
	uint8_t			buf[ 1 + inWriteLen ];
	size_t			len;
	
	deadline = UpTicks() + SecondsToUpTicks( 2 );
	if( inReadBuf )
	{
		// Combined mode transactions are not supported so set the register and do a separate read.
		
		for( tries = 1; ; ++tries )
		{
			n = write( inFD, &inRegister, 1 );
			err = map_global_value_errno( n == 1, n );
			if( !err ) break;
			
			dlog( kLogLevelVerbose, "### MFi auth set register 0x%02X failed (try %d): %#m\n", inRegister, tries, err );
			usleep( kMFiAuthRetryDelayMics );
			require_action( UpTicks() < deadline, exit, err = kTimeoutErr );
		}
		for( tries = 1; ; ++tries )
		{
			n = read( inFD, inReadBuf, inReadLen );
			err = map_global_value_errno( n == ( (ssize_t) inReadLen ), n );
			if( !err ) break;
			
			dlog( kLogLevelVerbose, "### MFi auth read register 0x%02X, %zu bytes failed (try %d): %#m\n", 
				inRegister, inReadLen, tries, err );
			usleep( kMFiAuthRetryDelayMics );
			require_action( UpTicks() < deadline, exit, err = kTimeoutErr );
		}
	}
	else
	{
		// Gather the register and data so it can be done as a single write transaction.
		
		buf[ 0 ] = inRegister;
		memcpy( &buf[ 1 ], inWritePtr, inWriteLen );
		len = 1 + inWriteLen;
		
		for( tries = 1; ; ++tries )
		{
			n = write( inFD, buf, len );
			err = map_global_value_errno( n == ( (ssize_t) len ), n );
			if( !err ) break;
			
			dlog( kLogLevelVerbose, "### MFi auth write register 0x%02X, %zu bytes failed (try %d): %#m\n", 
				inRegister, inWriteLen, tries, err );
			usleep( kMFiAuthRetryDelayMics );
			require_action( UpTicks() < deadline, exit, err = kTimeoutErr );
		}
	}
	
exit:
	if( err ) dlog( kLogLevelWarning, "### MFi auth register 0x%02X failed after %d tries: %#m\n", inRegister, tries, err );
	return( err );
}
