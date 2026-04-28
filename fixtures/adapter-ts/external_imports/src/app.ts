import React from 'react';
import { useState } from 'react';
import path from 'path';
import { createStore } from '@quantajs/core';
import { localHelper } from './utils';

export function main() {
    const result = localHelper();
    return result;
}
