import React from 'react';

type Props = {
  title: string;
};

export function Header(props: Props) {
  return <h1>{props.title}</h1>;
}
