provider "aws" {
  region = "us-east-1"
}

resource "aws_instance" "web" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.micro"

  tags = {
    Name = "web-server"
  }
}

resource "aws_instance" "api" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.large"

  tags = {
    Name = "api-server"
  }
}

resource "aws_ebs_volume" "data" {
  availability_zone = "us-east-1a"
  size              = 100
  type              = "gp3"

  tags = {
    Name = "data-volume"
  }
}
