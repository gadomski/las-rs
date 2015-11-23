#!/bin/bash

set -o errexit -o nounset

project="las"
rev=$(git rev-parse --short HEAD)

cargo doc
cd target/doc
echo "<meta http-equiv=refresh content=0;url=${project}/index.html>" > index.html

git init
git config user.name "Pete Gadomski"
git config user.email "pete.gadomski@gmail.com"

git remote add upstream "https://$GH_TOKEN@github.com/gadomski/las-rs"
git fetch upstream
git reset upstream/gh-pages

touch .

git add -A .
git commit -m "rebuild pages at ${rev}"
git push -q upstream HEAD:gh-pages
