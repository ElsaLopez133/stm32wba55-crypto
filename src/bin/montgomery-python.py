import math

# https://community.st.com/t5/stm32-mcus-products/stm32-pka-rsa-montgomery-parameter/td-p/648349

def generate_r2(n):
    n = int(n)
    k = n.bit_length()
    w = int(math.ceil(k/32))
    R = 2**(32*w)
    r2adj = 2**(w*32-k)
    Z = R - n*r2adj
    
    for i in range(0, w+2):
        Z = Z << 32
        MSW = (Z >> w*32) & (2**32-1)
        while (MSW != 0):
            Z = Z - (n * MSW * r2adj)
            MSW = (Z >> w*32) & (2**32-1)

    return (Z)

def int_to_big_endian_u32_array(value, num_words=8):
    result = [(value >> (32 * i)) & 0xFFFFFFFF for i in range(num_words)]
    return result[::-1]  # Reverse for big-endian order

def big_endian_u32_array_to_int(value):
    result = sum((value[i] << (32 * (len(value) - 1 - i))) for i in range(len(value)))
    return result


# Example usage:
# Convert N from a big-endian array of u32 values to an integer
# N_array = [
#     0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
#     0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
# ]
# N_array = [0xd]
# N_array = [0xf0000000, 0xd0000001]
N_array = [0xffffffff, 0x00000000, 0xffffffff, 0xffffffff, 0xbce6faad, 0xa7179e84, 0xf3b9cac2, 0xfc632551]

# Convert big-endian array to integer
# modulus = sum((N_array[i] << (32 * (len(N_array) - 1 - i))) for i in range(len(N_array)))
modulus = big_endian_u32_array_to_int(N_array)

# Compute R^2 mod N
r2_value = generate_r2(modulus)

# Convert R^2 to big-endian format
r2_array = int_to_big_endian_u32_array(r2_value, num_words=len(N_array))

print(f"R2: [u32; 8] = [{', '.join(f'0x{word:08X}' for word in r2_array)}];")
# print(f"R^2 mod N = {hex(r2_value)}")


