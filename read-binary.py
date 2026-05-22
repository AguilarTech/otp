# Assuming we have a file named "example_file.txt"
file_path = r"C:\Users\danie\Desktop\key\Daniel_Asd_202403030919"

# Open the file in binary mode and read the first byte
with open(file_path, 'rb') as file:
    first_byte = file.read(1)



decimal_value = int.from_bytes(first_byte, byteorder='big')

decimal_value

print(decimal_value)  # This will print the first byte of the filepythin